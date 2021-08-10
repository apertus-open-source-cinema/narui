use crate::{
    heart::{Context, Key},
    hooks::ContextListenable,
    KeyPart,
    ListenableGuard,
};
use std::marker::PhantomData;

pub struct EffectHandle<T> {
    key: Key,
    context: Context,
    phantom: PhantomData<T>,
}
impl<T> EffectHandle<T> {
    pub fn read(&self) -> ListenableGuard<T> {
        ListenableGuard {
            rw_lock_guard: self.context.global.read(),
            phantom: Default::default(),
            path: self.key,
        }
    }
}

pub trait ContextEffect {
    fn effect_key<T: Send + Sync + 'static>(
        &self,
        key: Key,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T>;
    fn effect<T: Send + Sync + 'static>(
        &self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T>;
}
impl ContextEffect for Context {
    fn effect_key<T: Send + Sync + 'static>(
        &self,
        key: Key,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T> {
        let deps_listenable = self.listenable_key(key.with(KeyPart::Deps), None);
        let some_deps = Some(deps);
        if *self.listen_ref(deps_listenable) != some_deps {
            self.shout(deps_listenable, some_deps);
            let handle = callback();
            self.global.write().tree.set(key, Box::new(handle))
        }
        EffectHandle { key, context: self.clone(), phantom: Default::default() }
    }

    fn effect<T: Send + Sync + 'static>(
        &self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<T> {
        self.effect_key(self.key_for_hook(), callback, deps)
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
