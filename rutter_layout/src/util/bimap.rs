use hashbrown::HashMap;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{BuildHasher, Hash},
};

#[derive(Debug)]
pub(crate) struct BiMap<A, B, Hasher = DefaultHasher> {
    a_to_b: HashMap<A, B, Hasher>,
    b_to_a: HashMap<B, A, Hasher>,
}

impl<A, B, H: Default> BiMap<A, B, H> {
    pub(crate) fn new() -> BiMap<A, B, H> {
        Self { a_to_b: HashMap::default(), b_to_a: HashMap::default() }
    }
}

impl<A: Hash + Eq + Clone, B: Hash + Eq + Clone, H: BuildHasher> BiMap<A, B, H> {
    pub(crate) fn contains_left(&self, key: &A) -> bool { self.a_to_b.contains_key(key) }

    pub(crate) fn insert(&mut self, left: A, right: B) {
        self.a_to_b.insert(left.clone(), right.clone());
        self.b_to_a.insert(right, left);
    }

    pub(crate) fn get_left(&self, left: &A) -> Option<&B> { self.a_to_b.get(left) }

    pub(crate) fn get_right(&self, right: &B) -> Option<&A> { self.b_to_a.get(right) }

    pub(crate) fn remove_left(&mut self, left: &A) -> Option<(A, B)> {
        self.a_to_b.remove(&left).map(|right| {
            let left = self.b_to_a.remove(&right).unwrap();
            (left, right)
        })
    }

    pub(crate) fn remove_right(&mut self, right: &B) -> Option<(A, B)> {
        self.b_to_a.remove(&right).map(|left| {
            let right = self.a_to_b.remove(&left).unwrap();
            (left, right)
        })
    }
}
