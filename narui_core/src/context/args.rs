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

    pub fn for_value(_: &T) -> Self { Default::default() }
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
    ($context:expr, $idx:ident, $($values:expr,)*) => {
        match $context.fragment_store.get_args_mut($idx) {
            None => $context.fragment_store.set_args($idx, $crate::_macro_api::smallvec![$(Box::new($values) as _,)*]),
            Some(old_values) => {
                let mut idx = 0;
                let mut any_changed = false;
                #[allow(unused)]
                fn constrain_type<T>(a: &mut T, b: &T) {}
                $({
                    let old_value = &mut old_values[idx];
                    let old = old_value.downcast_mut().expect("wrong type for argument");
                    constrain_type(old, &$values);
                    if !$crate::_macro_api::all_eq!(&*old, &$values) {
                        *old = $values;
                        any_changed = true;
                    }
                    idx += 1;
                })*
                if any_changed {
                    $context.fragment_store.set_args_dirty($idx);
                }
            }
        };
    };
}
pub use shout_args_ as shout_args;
