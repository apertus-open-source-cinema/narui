use crate::{heart::Key, hooks::*, KeyPart, ListenableGuard, PatchedTree, WidgetContext};
use std::{marker::PhantomData, sync::Arc};

#[derive(Debug, Clone)]
pub struct EffectHandle<T> {
    key: Key,
    tree: Arc<PatchedTree>,
    phantom: PhantomData<T>,
}
impl<T> EffectHandle<T> {
    pub fn read(&self) -> ListenableGuard<T> {
        ListenableGuard {
            entry: self.tree.get_unpatched(&self.key).unwrap(),
            phantom: Default::default(),
        }
    }
}

pub trait ContextEffect {
    fn effect_key<T: Send + Sync + 'static>(
        &mut self,
        key: Key,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T>;
    fn effect<T: Send + Sync + 'static>(
        &mut self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T>;
}

impl<'a> ContextEffect for WidgetContext<'a> {
    fn effect_key<T: Send + Sync + 'static>(
        &mut self,
        key: Key,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T> {
        let deps_listenable = self.listenable_key(key.with(KeyPart::Hook(0)), None);
        let deps = Some(deps);
        if *self.listen_ref(deps_listenable) != deps {
            self.tree.set_unconditional(deps_listenable.key, Box::new(deps));
            let handle = callback();
            self.tree.set_unconditional(key, Box::new(handle))
        }
        EffectHandle { key, tree: self.tree.clone(), phantom: Default::default() }
    }

    fn effect<T: Send + Sync + 'static>(
        &mut self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T> {
        let key = self.key_for_hook();
        self.effect_key(key, callback, deps)
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
