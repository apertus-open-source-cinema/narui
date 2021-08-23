mod bimap;
pub(crate) use bimap::*;

mod vec_with_holes;
use std::num::NonZeroUsize;
pub(crate) use vec_with_holes::*;

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub(crate) struct Idx(NonZeroUsize);

impl Idx {
    pub(crate) fn new(val: usize) -> Self { Idx(unsafe { NonZeroUsize::new_unchecked(val + 1) }) }

    pub(crate) fn get(&self) -> usize { self.0.get() - 1 }
}
