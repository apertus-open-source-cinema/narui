use crate::{hooks::*, HookKey, HookRef, ListenableGuard, PatchedTree, WidgetContext};
use std::{marker::PhantomData, sync::Arc};

#[derive(Debug, Clone)]
pub struct EffectHandle<T> {
    key: HookRef,
    tree: Arc<PatchedTree>,
    phantom: PhantomData<T>,
}
impl<T> EffectHandle<T> {
    pub fn read(&self) -> ListenableGuard<T> {
        ListenableGuard { entry: self.tree.get_unpatched(self.key), phantom: Default::default() }
    }
}

pub trait ContextEffect {
    fn effect<T: Send + Sync + 'static>(
        &mut self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T>;
}

impl<'a> ContextEffect for WidgetContext<'a> {
    fn effect<T: Send + Sync + 'static>(
        &mut self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T> {
        let deps_listenable = self.listenable(None);
        let handle_listenable = self.listenable_with(|| callback());
        let deps = Some(deps);
        if *self.listen_ref(deps_listenable) != deps {
            self.tree.set_unconditional(deps_listenable.key.1, Box::new(deps));
            let handle = callback();
            self.tree.set_unconditional(handle_listenable.key.1, Box::new(handle))
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
