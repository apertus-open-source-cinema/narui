use crate::heart::{Key, Context};
use crate::hooks::ContextListenable;

pub trait ContextEffect {
    fn effect_key(&self, key: Key, callback: impl Fn() -> (), deps: impl PartialEq);
    fn effect(&self, callback: impl Fn() -> (), deps: impl PartialEq);
}
impl ContextEffect for Context {
    fn effect_key(&self, key: Key, callback: impl Fn(), deps: impl PartialEq) {
        let listenable = self.listenable_key(key, None);
        if self.listen(&listenable) != deps {
            self.shout(&listenable, Some(deps));
            callback();
        }
    }

    fn effect(&self, callback: impl Fn(), deps: impl PartialEq) {
        self.effect_key(self.key_for_hook(), callback, deps)
    }
}