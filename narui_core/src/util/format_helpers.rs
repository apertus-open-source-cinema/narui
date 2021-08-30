use std::fmt::{Formatter, Result};

pub(crate) fn print_vec_len<T>(vec: &[T], fmt: &mut Formatter) -> Result {
    write!(fmt, "Vec {{ len() = {} }}", vec.len())
}
