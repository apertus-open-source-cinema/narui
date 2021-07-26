use crate::{
    heart::{Context, Key, KeyPart},
    hooks::{ContextListenable, Listenable},
};

pub trait ContextMemoize {
    fn memoize_key<T: Send + Sync + 'static>(
        &self,
        key: Key,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + Clone + 'static + PartialEq,
    ) -> Listenable<T>;
    fn memoize<T: Send + Sync + 'static>(
        &self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + Clone + 'static + PartialEq,
    ) -> Listenable<T>;
}
impl ContextMemoize for Context {
    fn memoize_key<T: Send + Sync + 'static>(
        &self,
        key: Key,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + Clone + 'static + PartialEq,
    ) -> Listenable<T> {
        let last_deps = self.listenable_key(key.with(KeyPart::Deps), deps.clone());
        let last_result = self.listenable_key(key, callback());
        if *self.listen_ref(last_deps) != deps {
            self.shout(last_deps, deps);
            self.shout_unconditional(last_result, callback());
        }
        last_result
    }

    fn memoize<T: Send + Sync + 'static>(
        &self,
        callback: impl Fn() -> T,
        deps: impl PartialEq + Send + Sync + Clone + 'static + PartialEq,
    ) -> Listenable<T> {
        self.memoize_key(self.key_for_hook(), callback, deps)
    }
}
