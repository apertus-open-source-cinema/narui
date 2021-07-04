// this hook is for internal use only !
// it stores the arguments of each widget function in the state tree for being
// able to depend on them in delta evaluation and trigger re-eval when they
// change.

use crate::{
    heart::{Context, KeyPart},
    hooks::ContextListenable,
};

pub trait ContextArgs {
    fn args<T: Clone + Send + Sync + 'static>(&self, args: &T);
}

impl ContextArgs for Context {
    fn args<T: Clone + Send + Sync + 'static>(&self, args: &T) {
        let args_listenable = self.listenable_key(self.widget_local.key.with(KeyPart::Args), None);
        self.shout(args_listenable, Some(args.clone()));
        self.listen(args_listenable);
    }
}
