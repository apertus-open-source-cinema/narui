use crate::heart::{Key, Context};
use crate::hooks::{ContextListenable, Listenable};

pub trait ContextMemoize {
    fn memoize_key<T>(&self, key: Key, callback: impl Fn() -> T, deps: impl PartialEq) -> T;
    fn memoize<T>(&self, callback: impl Fn() -> T, deps: impl PartialEq) -> T;
}
impl ContextMemoize for Context {
    fn memoize_key<T>(&self, key: Key, callback: impl Fn() -> T, deps: impl PartialEq) -> Listenable<T> {
        let last_deps = self.listenable_key(key, None);
        let last_result = self.listenable_key(key, None);
        if self.listen(&last_deps) != deps {
            self.shout(&last_deps, Some(deps));
            self.shout(&last_result, callback());
        }
        last_result
    }

    fn memoize<T>(&self, callback: impl Fn() -> T, deps: impl PartialEq) -> Listenable<T> {
        self.memoize_key(self.key_for_hook(), callback, deps)
    }
}