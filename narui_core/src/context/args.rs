use crate::{Fragment, WidgetContext};
use derivative::Derivative;

use smallvec::SmallVec;
use std::{any::Any, marker::PhantomData};

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

pub fn listen_args<'a>(
    context: &'a mut WidgetContext,
    key: &Fragment,
) -> &'a SmallVec<[Box<dyn Any>; 8]> {
    context.fragment_store.get_args(*key).as_ref().unwrap()
}

/// shout arg is a macro for not requiring the implementation of PartialEq on
/// all args.
#[macro_export]
macro_rules! shout_args_ {
    ($context:expr, $idx:ident, [$($values:ident,)*]) => {{
        let mut arg_refs = ($idx,
            $({
                let arg_ref = $crate::_macro_api::ArgRef::new();
                #[allow(unused)]
                pub fn constrain_type<T>(_arg_ref: $crate::_macro_api::ArgRef<T>, _value: &T) {};
                constrain_type(arg_ref, &$values);
                arg_ref
            }),*
        );

        match $context.fragment_store.get_args_mut($idx) {
            None => $context.fragment_store.set_args($idx, $crate::_macro_api::smallvec![$(Box::new($values) as _,)*]),
            Some(old_values) => {
                let mut idx = 0;
                let mut any_changed = false;
                #[allow(unused)]
                fn constrain_type<T>(_a: &T, _b: &T) {}
                $({
                    let old_value = &mut old_values[idx];
                    let old = (&mut **old_value).downcast_mut().expect(
                        "old value of arg has wrong type; this is likely an internal narui bug :(",
                    );
                    constrain_type(old, &$values);
                    let changed = !$crate::_macro_api::all_eq!(&*old, &$values);

                    if changed {
                        *old = $values;
                    }

                    any_changed = any_changed || changed;

                    idx += 1;
                })*
                if any_changed {
                    $context.fragment_store.set_args_dirty($idx);
                }
            }
        };

        arg_refs
    }};
}
pub use shout_args_ as shout_args;
