use super::{ListenableCreate, ListenableListen};
use crate::{
    context::patched_tree::{HookRef, PatchedTree},
    Listenable,
    MappedListenableGuard,
    WidgetContext,
};
use std::{marker::PhantomData, sync::Arc};

#[derive(Debug, Clone)]
pub struct EffectHandle<T> {
    key: HookRef,
    tree: Arc<PatchedTree>,
    phantom: PhantomData<T>,
}
impl<T> EffectHandle<T> {
    pub fn read(
        &self,
    ) -> MappedListenableGuard<Option<T>, T, impl for<'a> Fn(&'a Option<T>) -> &'a T> {
        MappedListenableGuard {
            entry: self.tree.get_unpatched(self.key),
            mapping_function: |elem: &Option<T>| elem.as_ref().unwrap(),
            phantom: Default::default(),
            phantom2: Default::default(),
        }
    }
}

pub trait ContextEffect {
    fn effect<T: Send + Sync + 'static>(
        &mut self,
        callback: impl Fn(&mut WidgetContext) -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T>;
}

impl<'a> ContextEffect for WidgetContext<'a> {
    fn effect<T: Send + Sync + 'static>(
        &mut self,
        callback: impl FnOnce(&mut WidgetContext) -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T> {
        let deps_listenable = self.listenable(None);
        let handle_listenable: Listenable<Option<T>> = self.listenable(None);
        let deps = Some(deps);

        if self.listen_ref(handle_listenable).is_none() {
            let handle = callback(self);
            self.tree.set_unconditional(handle_listenable.key.1, Box::new(Some(handle)));
            self.tree.set_unconditional(deps_listenable.key.1, Box::new(deps));
        } else if *self.listen_ref(deps_listenable) != deps {
            self.tree.set_unconditional(deps_listenable.key.1, Box::new(deps));
            let handle = callback(self);
            self.tree.set_unconditional(handle_listenable.key.1, Box::new(Some(handle)))
        }

        EffectHandle {
            key: handle_listenable.key,
            tree: self.tree.clone(),
            phantom: Default::default(),
        }
    }
}

pub struct DropCallbackHelper<T: FnOnce()> {
    callback: Option<T>,
}
impl<T: FnOnce()> DropCallbackHelper<T> {
    pub fn new(callback: T) -> DropCallbackHelper<T> { Self { callback: Some(callback) } }
}
impl<T: FnOnce()> Drop for DropCallbackHelper<T> {
    fn drop(&mut self) { (self.callback.take().unwrap())() }
}
