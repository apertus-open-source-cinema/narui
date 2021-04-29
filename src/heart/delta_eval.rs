use std::hash::{Hasher, Hash};
use std::any::type_name;

// ideom stolen from https://github.com/nvzqz/impls
pub trait NotHash {
    fn hash_all<T>(&self, input: &T, state: &mut impl Hasher) {
        let size = std::mem::size_of::<T>();
        let arr = unsafe {std::slice::from_raw_parts(std::mem::transmute::<&T, *const u8>(input), size)};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        arr.hash(&mut hasher);
        println!("fallback_hash\t{}\t(size: {}; hash: {})", dbg_type(input), size, hasher.finish());
        arr.hash(state);
    }
}
impl<T> NotHash for T {}

pub struct IsHash<T>(pub std::marker::PhantomData<T>);
impl<T: Hash> IsHash<T> {
    pub fn hash_all(&self, input: &T, state: &mut impl Hasher) {
        println!("real_hash\t{}", dbg_type(input));

        input.hash(state)
    }
}

fn dbg_type<T>(_: &T) -> &str {
    type_name::<T>()
}
