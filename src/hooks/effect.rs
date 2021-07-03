use crate::{
    heart::{Context, Key},
    hooks::ContextListenable,
};

pub trait ContextEffect {
    fn effect_key(
        &self,
        key: Key,
        callback: impl Fn(),
        deps: impl PartialEq + Send + Sync + 'static,
    );
    fn effect(&self, callback: impl Fn(), deps: impl PartialEq + Send + Sync + 'static);
}
impl ContextEffect for Context {
    fn effect_key(
        &self,
        key: Key,
        callback: impl Fn(),
        deps: impl PartialEq + Send + Sync + 'static,
    ) {
        let listenable = self.listenable_key(key, None);
        let some_deps = Some(deps);
        if *self.listen_ref(listenable) != some_deps {
            self.shout(listenable, some_deps);
            callback();
        }
    }

    fn effect(&self, callback: impl Fn(), deps: impl PartialEq + Send + Sync + 'static) {
        self.effect_key(self.key_for_hook(), callback, deps)
    }
}
