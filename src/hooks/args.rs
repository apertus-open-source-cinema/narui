use crate::{Key, WidgetContext};
use derivative::Derivative;
pub use freelist::Idx;
use std::{any::Any, marker::PhantomData};

pub trait ContextArgs {
    fn listen_args(&self, key: &Idx) -> &Vec<Box<dyn Any>>;
}


impl<'a> ContextArgs for WidgetContext<'a> {
    fn listen_args(&self, key: &Idx) -> &Vec<Box<dyn Any>> {
        self.args_tree.get_unconditional(*key)
    }
}

#[derive(Debug, Derivative)]
#[derivative(Default(bound = "", new = "true"))]
pub struct ArgRef<T> {
    marker: PhantomData<T>,
}

impl<T> Clone for ArgRef<T> {
    fn clone(&self) -> Self { Self::new() }
}
impl<T> Copy for ArgRef<T> {}

impl<T> ArgRef<T> {
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
        let mut arg_refs = (narui::args::Idx::new(1).unwrap(),
            $({
                let arg_ref = narui::args::ArgRef::new();
                #[allow(unused)]
                pub fn constrain_type<T>(_arg_ref: narui::args::ArgRef<T>, _value: &T) {};
                constrain_type(arg_ref, &$values);
                arg_ref
            }),*
        );

        let idx = match $context.args_tree.get_idx($key) {
            None => $context.args_tree.set($key, vec![$(Box::new($values),)*]),
            Some(old_idx) => {
                let old_values = $context.args_tree.get_unconditional(old_idx);
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
                    $context.args_tree.set_unconditional($key, old_idx, vec![$(Box::new($values),)*]);
                }
                old_idx
            }
        };

        arg_refs.0 = idx;

        arg_refs
    }};
}
