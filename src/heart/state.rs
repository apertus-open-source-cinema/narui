use hashbrown::HashMap;
use std::{
    any::Any,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};

// TODO: this seems to be not permant... Investigate
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyInner {
    parent: Option<Key>,
    own: String,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(Arc<KeyInner>);
impl Key {
    pub fn new(s: &str) -> Self { Key(Arc::new(KeyInner { parent: None, own: s.to_string() })) }

    pub fn enter(&self, s: &str) -> Self {
        Key(Arc::new(KeyInner { parent: Some(self.clone()), own: s.to_string() }))
    }
}

pub type TreeStateInner = HashMap<Key, Box<dyn Any>>;
#[derive(Clone, Debug, Default)]
pub struct TreeState(pub Arc<RwLock<TreeStateInner>>);

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub key: Key,
    pub tree: TreeState,
}
impl Context {
    pub fn enter(&self, key: &str) -> Context {
        Context { key: self.key.enter(key), tree: self.tree.clone() }
    }
    pub fn ident(&self) -> *const Box<dyn Any> {
        &self.tree.0.read().unwrap()[&self.key] as *const Box<dyn Any>
    }
}

#[derive(Clone, Debug)]
pub struct StateValue<T> {
    pub context: Context,
    phantom: PhantomData<T>,
}
impl<T> StateValue<T> {
    pub fn new(context: Context, key: &str) -> Self {
        StateValue { context: context.enter(key), phantom: PhantomData::default() }
    }
}
impl<T: 'static + Sync + Send> StateValue<T> {
    pub fn is_present(&self) -> bool {
        self.context.tree.0.read().unwrap().contains_key(&self.context.key)
    }
    pub fn set(&self, new_value: T) {
        self.context.tree.0.write().unwrap().insert(self.context.key.clone(), Box::new(new_value));
    }
    pub fn get_ref(&self) -> StateValueGuard<T> {
        StateValueGuard {
            rw_lock_guard: self.context.tree.0.read().unwrap(),
            path: self.context.key.clone(),
            phantom: Default::default(),
        }
    }
}
impl<T: Clone + 'static> StateValue<T> {
    pub fn get(&self) -> T {
        self.context.tree.0.read().unwrap()[&self.context.key].downcast_ref::<T>().unwrap().clone()
    }
}

pub struct StateValueGuard<'l, T> {
    rw_lock_guard: RwLockReadGuard<'l, TreeStateInner>,
    phantom: PhantomData<T>,
    path: Key,
}
impl<'l, T: 'static> Deref for StateValueGuard<'l, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { self.rw_lock_guard[&self.path].downcast_ref().unwrap() }
}
