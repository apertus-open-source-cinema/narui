use crate::heart::{Context, Key};
use parking_lot::RwLockReadGuard;
use std::{marker::PhantomData, ops::Deref};


pub trait ContextListenable {
    fn listenable_key<T: Send + Sync + 'static>(&self, key: Key, initial: T) -> Listenable<T>;
    fn listenable<T: Send + Sync + 'static>(&self, initial: T) -> Listenable<T>;

    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T);
    fn shout_unconditional<T: Send + Sync + 'static>(&self, listenable: Listenable<T>, initial: T);

    fn listen<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone;
    fn listen_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T>;
}
impl ContextListenable for Context {
    fn listenable_key<T: Send + Sync + 'static>(&self, key: Key, initial: T) -> Listenable<T> {
        let listenable = Listenable { key, phantom_data: Default::default() };
        if self.global.read().tree.get(listenable.key).is_none() {
            self.shout_unconditional(listenable, initial)
        };
        listenable
    }
    fn listenable<T: Send + Sync + 'static>(&self, initial: T) -> Listenable<T> {
        self.listenable_key(self.key_for_hook(), initial)
    }

    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T) {
        let mut lock = self.global.write();
        match lock.tree.get(listenable.key) {
            None => lock.tree.set(listenable.key, Box::new(new_value)),
            Some(old_value) => {
                if (&**old_value).downcast_ref::<T>().expect("old value has wrong type")
                    != &new_value
                {
                    lock.tree.set(listenable.key, Box::new(new_value))
                }
            }
        }
    }
    fn shout_unconditional<T: Send + Sync + 'static>(
        &self,
        listenable: Listenable<T>,
        new_value: T,
    ) {
        let mut lock = self.global.write();
        lock.tree.set(listenable.key, Box::new(new_value))
    }

    fn listen<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone,
    {
        self.widget_local.mark_used(listenable.key);
        self.global
            .read()
            .tree
            .get(listenable.key)
            .expect("cant find key of listenable in Context")
            .downcast_ref::<T>()
            .expect("Listenable has wrong type")
            .clone()
    }

    fn listen_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T> {
        ListenableGuard {
            rw_lock_guard: self.global.read(),
            path: listenable.key,
            phantom: Default::default(),
        }
    }
}

#[macro_export]
macro_rules! shout_ {
    ($context:ident, $listenable:ident, $value:expr) => {{
        fn constrain_type<T>(_a: &Listenable<T>, _b: &T) {}
        constrain_type(&$listenable, &$value);
        let mut lock = $context.global.write();
        match lock.tree.get($listenable.key) {
            None => lock.tree.set($listenable.key, Box::new($value)),
            Some(old_value) => {
                let old = (&**old_value).downcast_ref().expect("old value has wrong type");
                fn constrain_type<T>(_a: &T, _b: &T) {}
                constrain_type(old, &$value);
                if !all_eq!(old, &$value) {
                    lock.tree.set($listenable.key, Box::new($value));
                }
            }
        }
    }};
}
use crate::ApplicationGlobalContext;
pub use shout_ as shout;

pub struct Listenable<T> {
    pub key: Key,
    phantom_data: PhantomData<T>,
}
impl<T> Listenable<T> {
    pub unsafe fn uninitialized(key: Key) -> Self {
        Listenable { key, phantom_data: Default::default() }
    }
}
impl<T> Clone for Listenable<T> {
    fn clone(&self) -> Self { Self { key: self.key, phantom_data: Default::default() } }
}
impl<T> Copy for Listenable<T> {}

pub struct ListenableGuard<'l, T> {
    pub(crate) rw_lock_guard: RwLockReadGuard<'l, ApplicationGlobalContext>,
    pub(crate) phantom: PhantomData<T>,
    pub(crate) path: Key,
}
impl<'l, T: 'static> Deref for ListenableGuard<'l, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.rw_lock_guard
            .tree
            .get(self.path)
            .expect("cant find key of ListenableGuard in Context")
            .downcast_ref()
            .expect("ListenableGuard has wrong type")
    }
}
