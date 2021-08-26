use crate::{Key, WidgetContext};
use std::any::Any;

pub trait ContextArgs {
    fn listen_args(&self, key: &Key) -> &Vec<Box<dyn Any>>;
}


impl<'a> ContextArgs for WidgetContext<'a> {
    fn listen_args(&self, key: &Key) -> &Vec<Box<dyn Any>> {
        self.args_tree.get(key).unwrap_or_else(|| {
            panic!(
                "args key not present; this is likely an internal narui bug :(, {:?}, {:#?}",
                &key, self.args_tree
            )
        })
    }
}

// shout arg is a macro for not requiring the implementation of PartialEq on all
// args.
#[macro_export]
macro_rules! shout_args {
    ($context:expr, $key:expr, [$($values:ident,)*]) => {{
        let listenables = ($key,
            $({
                let listenable = unsafe { Listenable::uninitialized($key) };
                pub fn constrain_type<T>(_listenable: Listenable<T>, _value: &T) {};
                constrain_type(listenable, &$values);
                listenable
            }),*
        );

        match $context.args_tree.get(&$key) {
            None => $context.args_tree.set($key, vec![$(Box::new($values),)*]),
            Some(old_values) => {
                let mut changed = false;
                let mut idx = 0;
                fn constrain_type<T>(_a: &T, _b: &T) {}
                for i in 0..1 {
                    $(
                        let old_value = &old_values[idx];
                        let old = (&**old_value).downcast_ref().expect(
                            "old value of arg has wrong type; this is likely an internal narui bug :(",
                        );
                        constrain_type(old, &$values);
                        if !all_eq!(old, &$values) {
                            changed = true;
                            break;
                        }
                        idx += 1;
                    )*
                }
                if changed {
                    $context.args_tree.set($key, vec![$(Box::new($values),)*]);
                }
            }
        }

        listenables
    }};
}
