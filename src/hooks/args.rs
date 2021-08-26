use crate::{Key, WidgetContext};
use std::any::Any;
use std::marker::PhantomData;

pub trait ContextArgs {
    fn listen_args(&self, key: &Key) -> &Vec<Box<dyn Any>>;
}


impl<'a> ContextArgs for WidgetContext<'a> {
    fn listen_args(&self, key: &Key) -> &Vec<Box<dyn Any>> {
        self.args_tree.get(key).unwrap_or_else(|| {
            panic!(
                "args key not present; this is likely an internal narui bug :(, {:?}, {}",
                &self.key_map.key_debug(*key), self.args_tree.debug_print(self.key_map)
            )
        })
    }
}

pub struct ArgRef<T> {
    marker: PhantomData<T>
}

impl<T> Clone for ArgRef<T> {
    fn clone(&self) -> Self { Self::new() }
}
impl<T> Copy for ArgRef<T> {}

impl<T> ArgRef<T> {
    pub fn new() -> Self {
        Self {
            marker: PhantomData
        }
    }

    pub unsafe fn parse<'a>(&self, any: &'a dyn Any) -> &'a T
        where
            T: 'static,
    {
        any.downcast_ref().expect("wrong type for argument")
    }
}

// shout arg is a macro for not requiring the implementation of PartialEq on all
// args.
#[macro_export]
macro_rules! shout_args {
    ($context:expr, $key:expr, [$($values:ident,)*]) => {{
        let arg_refs = ($key,
            $({
                let arg_ref = narui::args::ArgRef::new();
                #[allow(unused)]
                pub fn constrain_type<T>(_arg_ref: narui::args::ArgRef<T>, _value: &T) {};
                constrain_type(arg_ref, &$values);
                arg_ref
            }),*
        );

        match $context.args_tree.get(&$key) {
            None => $context.args_tree.set($key, vec![$(Box::new($values),)*]),
            Some(old_values) => {
                let mut changed = false;
                let mut idx = 0;
                #[allow(unused)]
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

        arg_refs
    }};
}
