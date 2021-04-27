use hashbrown::HashMap;
use std::{
    any::Any,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum KeyInner {
    Root,
    Sideband { key: String },
    StateValue { parent: Key, key: String },
    Widget { parent: Key, name: &'static str, loc: &'static str },
    WidgetKey { parent: Key, name: &'static str, loc: &'static str, key: String },
    Hook { parent: Key, name: &'static str, loc: &'static str },
}
impl Default for KeyInner {
    fn default() -> Self { Self::Root }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(Arc<KeyInner>);
impl Key {
    pub fn sideband(key: String) -> Self { Key(Arc::new(KeyInner::Sideband { key })) }

    pub fn enter_state_value(&self, key: String) -> Self {
        Key(Arc::new(KeyInner::StateValue { parent: self.clone(), key }))
    }
    pub fn enter_widget(&self, name: &'static str, loc: &'static str) -> Self {
        Key(Arc::new(KeyInner::Widget { parent: self.clone(), name, loc }))
    }
    pub fn enter_widget_key(&self, name: &'static str, loc: &'static str, key: String) -> Self {
        Key(Arc::new(KeyInner::WidgetKey { parent: self.clone(), name, loc, key }))
    }
    pub fn enter_hook(&self, name: &'static str, loc: &'static str) -> Self {
        Key(Arc::new(KeyInner::Hook { parent: self.clone(), name, loc }))
    }
}

type TreeStateInner = HashMap<Key, Box<dyn Any>>;
type TreeState = Arc<RwLock<TreeStateInner>>;

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub key: Key,
    pub tree: TreeState,
}
impl Context {
    pub fn sideband(&self, key: String) -> Self {
        Context { key: Key(Arc::new(KeyInner::Sideband { key })), tree: self.tree.clone() }
    }
    pub fn enter_state_value(&self, key: String) -> Self {
        Context { key: self.key.enter_state_value(key), tree: self.tree.clone() }
    }
    pub fn enter_widget(&self, name: &'static str, loc: &'static str) -> Self {
        Context { key: self.key.enter_widget(name, loc), tree: self.tree.clone() }
    }
    pub fn enter_widget_key(&self, name: &'static str, loc: &'static str, key: String) -> Self {
        Context { key: self.key.enter_widget_key(name, loc, key), tree: self.tree.clone() }
    }
    pub fn enter_hook(&self, name: &'static str, loc: &'static str) -> Self {
        Context { key: self.key.enter_hook(name, loc), tree: self.tree.clone() }
    }

    pub fn ident(&self) -> *const Box<dyn Any> {
        &self.tree.read().unwrap()[&self.key] as *const Box<dyn Any>
    }
    pub fn is_present(&self) -> bool { self.tree.read().unwrap().contains_key(&self.key) }
}

#[derive(Clone, Debug)]
pub struct StateValue<T> {
    pub context: Context,
    phantom: PhantomData<T>,
}
impl<T> StateValue<T> {
    pub fn new(context: Context, key: &str) -> Self {
        StateValue {
            context: context.enter_state_value(key.to_string()),
            phantom: PhantomData::default(),
        }
    }
}
impl<T: 'static + Sync + Send> StateValue<T> {
    pub fn set(&self, new_value: T) {
        self.context.tree.write().unwrap().insert(self.context.key.clone(), Box::new(new_value));
    }
    pub fn get_ref(&self) -> StateValueGuard<T> {
        StateValueGuard {
            rw_lock_guard: self.context.tree.read().unwrap(),
            path: self.context.key.clone(),
            phantom: Default::default(),
        }
    }
}
impl<T: Clone + 'static> StateValue<T> {
    pub fn get(&self) -> T {
        self.context.tree.read().unwrap()[&self.context.key].downcast_ref::<T>().unwrap().clone()
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
