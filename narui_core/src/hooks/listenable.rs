use crate::{
    context::{HookRef, PatchTreeEntry, PatchedTree},
    CallbackContext,
    ThreadContext,
    WidgetContext,
};
use std::{marker::PhantomData, ops::Deref};

pub trait ListenableCreate {
    fn listenable_with<T: Send + Sync + 'static>(
        &mut self,
        gen: impl FnOnce() -> T,
    ) -> Listenable<T>;
    fn listenable<T: Send + Sync + 'static>(&mut self, initial: T) -> Listenable<T>;
}

pub trait ListenableShout {
    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T);
    fn shout_non_signalling<T: Send + Sync + 'static>(&self, listenable: Listenable<T>, initial: T);
}

pub trait ListenableListen {
    fn listen<T: Send + Sync + 'static>(&mut self, listenable: Listenable<T>) -> T
    where
        T: Clone;
    fn listen_ref<T: Send + Sync>(&mut self, listenable: Listenable<T>) -> ListenableGuard<T>;
}

pub trait ListenableSpy {
    fn spy<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone;

    fn spy_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T>;
}

impl<'a> ListenableCreate for WidgetContext<'a> {
    fn listenable_with<T: Send + Sync + 'static>(
        &mut self,
        gen: impl FnOnce() -> T,
    ) -> Listenable<T> {
        let key = self.key_for_hook();
        let key = self.tree.initialize_with(key, || Box::new(gen()));
        Listenable { key, phantom_data: Default::default() }
    }

    fn listenable<T: Send + Sync + 'static>(&mut self, initial: T) -> Listenable<T> {
        let key = self.key_for_hook();
        let key = self.tree.initialize_with(key, || Box::new(initial));
        Listenable { key, phantom_data: Default::default() }
    }
}

impl ListenableShout for PatchedTree {
    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T) {
        let old_entry = self.get_unpatched(listenable.key);
        let old = (&**old_entry)
            .downcast_ref::<T>()
            .expect("old value of arg has wrong type; this is likely an internal narui bug :(");
        if old != &new_value {
            std::mem::drop(old_entry);
            self.set(listenable.key, Box::new(new_value));
        } else {
            // it is possible that there was a previous patch, but now we set this
            // listenable to the original (unpatched) value again so we have to
            // remove the potential patch
            self.remove_patch(listenable.key);
        }
    }

    fn shout_non_signalling<T: Send + Sync + 'static>(
        &self,
        listenable: Listenable<T>,
        new_value: T,
    ) {
        self.set_unconditional(listenable.key.1, Box::new(new_value))
    }
}

impl ListenableShout for ThreadContext {
    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T) {
        self.tree.shout(listenable, new_value);
    }

    fn shout_non_signalling<T: Send + Sync + 'static>(
        &self,
        listenable: Listenable<T>,
        new_value: T,
    ) {
        self.tree.shout_non_signalling(listenable, new_value)
    }
}

impl ListenableShout for CallbackContext<'_> {
    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T) {
        self.tree.shout(listenable, new_value);
    }

    fn shout_non_signalling<T: Send + Sync + 'static>(
        &self,
        listenable: Listenable<T>,
        new_value: T,
    ) {
        self.tree.shout_non_signalling(listenable, new_value)
    }
}

impl ListenableSpy for PatchedTree {
    fn spy<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone,
    {
        self.get_patched(listenable.key)
            .downcast_ref::<T>()
            .expect("Listenable has wrong type")
            .clone()
    }

    fn spy_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T> {
        ListenableGuard::new(self.get_patched(listenable.key))
    }
}

impl ListenableSpy for CallbackContext<'_> {
    fn spy<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone,
    {
        self.tree.spy(listenable)
    }

    fn spy_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T> {
        self.tree.spy_ref(listenable)
    }
}

impl ListenableSpy for ThreadContext {
    fn spy<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone,
    {
        self.tree.spy(listenable)
    }

    fn spy_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T> {
        self.tree.spy_ref(listenable)
    }
}

impl<'a> ListenableListen for WidgetContext<'a> {
    fn listen<T: Send + Sync + 'static>(&mut self, listenable: Listenable<T>) -> T
    where
        T: Clone,
    {
        self.tree.set_dependent(listenable.key, self.widget_local.idx);
        self.tree
            .get_unpatched(listenable.key)
            .downcast_ref::<T>()
            .expect("Listenable has wrong type")
            .clone()
    }

    fn listen_ref<T: Send + Sync>(&mut self, listenable: Listenable<T>) -> ListenableGuard<T> {
        // TODO(robin): why was this previously not marked as used?
        self.tree.set_dependent(listenable.key, self.widget_local.idx);

        ListenableGuard::new(self.tree.get_unpatched(listenable.key))
    }
}


pub struct Listenable<T> {
    pub key: HookRef,
    phantom_data: PhantomData<T>,
}
impl<T> Clone for Listenable<T> {
    fn clone(&self) -> Self { Self { key: self.key, phantom_data: Default::default() } }
}
impl<T> Copy for Listenable<T> {}

pub struct ListenableGuard<'a, T> {
    pub(crate) entry: PatchTreeEntry<'a>,
    pub(crate) phantom: PhantomData<T>,
}

impl<'a, T> ListenableGuard<'a, T> {
    fn new(entry: PatchTreeEntry<'a>) -> Self { Self { entry, phantom: Default::default() } }
}

impl<'a, T: 'static> Deref for ListenableGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.entry.downcast_ref().expect("ListenableGuard has wrong type")
    }
}

pub struct MappedListenableGuard<'a, T, MT, FN: Fn(&T) -> &MT> {
    pub(crate) entry: PatchTreeEntry<'a>,
    pub(crate) mapping_function: FN,
    pub(crate) phantom: PhantomData<T>,
    pub(crate) phantom2: PhantomData<MT>,
}

impl<'a, T: 'static, FN, MT> Deref for MappedListenableGuard<'a, T, MT, FN>
where
    FN: Fn(&T) -> &MT,
{
    type Target = MT;

    fn deref(&self) -> &Self::Target {
        (self.mapping_function)(
            &*self.entry.downcast_ref().expect("ListenableGuard has wrong type"),
        )
    }
}
