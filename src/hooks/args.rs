use crate::{Key, Listenable, WidgetContext};

pub trait ContextArgs {
    fn listen_arg<T: 'static + Clone>(&mut self, listenable: Listenable<T>) -> T;
}


impl<'a> ContextArgs for WidgetContext<'a> {
    fn listen_arg<T: 'static + Clone>(&mut self, listenable: Listenable<T>) -> T {
        self.args_tree
            .get(&listenable.key)
            .expect("args key not present; this is likely an internal narui bug :(")
            .downcast_ref::<T>()
            .expect("args key has unexpected type; this is likely an internal narui bug :(")
            .clone()
    }
}

// shout arg is a macro for not requiring the implementation of PartialEq on all
// args.
#[macro_export]
macro_rules! shout_arg {
    ($context:expr, $key:expr, $value:ident) => {{
        let listenable = unsafe { Listenable::uninitialized($key) };
        pub fn constrain_type<T>(_listenable: Listenable<T>, _value: &T) {};
        constrain_type(listenable, &$value);
        match $context.args_tree.get(&$key) {
            None => $context.args_tree.set($key, Box::new($value)),
            Some(old_value) => {
                let old = (&**old_value).downcast_ref().expect(
                    "old value of arg has wrong type; this is likely an internal narui bug :(",
                );
                fn constrain_type<T>(_a: &T, _b: &T) {}
                constrain_type(old, &$value);
                if !all_eq!(old, &$value) {
                    $context.args_tree.set(listenable.key, Box::new($value));
                }
            }
        }
        listenable
    }};
}
