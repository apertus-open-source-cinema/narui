use hashbrown::{HashMap, HashSet};
use std::{
    any::Any,
    fmt,
    fmt::{Debug, Formatter},
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc},
};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use std::panic::panic_any;

#[derive(Hash, Eq, PartialEq, Clone, Ord, PartialOrd)]
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
impl Debug for KeyInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            KeyInner::Root => {}
            KeyInner::Sideband { key } => {
                f.write_str("[sideband:")?;
                f.write_str(key)?;
                f.write_str("] ")?;
            }
            KeyInner::StateValue { parent, key } => {
                parent.0.fmt(f)?;
                f.write_str("[state_value:")?;
                f.write_str(key)?;
                f.write_str("] ")?;
            }
            KeyInner::Widget { parent, name, loc } => {
                parent.0.fmt(f)?;
                f.write_str("[widget:")?;
                f.write_str(name)?;
                f.write_str("@")?;
                f.write_str(loc)?;
                f.write_str("] ")?;
            }
            KeyInner::WidgetKey { parent, name, loc, key } => {
                parent.0.fmt(f)?;
                f.write_str("[widget:")?;
                f.write_str(name)?;
                f.write_str("@")?;
                f.write_str(loc)?;
                f.write_str("(")?;
                f.write_str(key)?;
                f.write_str(")")?;
                f.write_str("] ")?;
            }
            KeyInner::Hook { parent, name, loc } => {
                parent.0.fmt(f)?;
                f.write_str("[hook:")?;
                f.write_str(name)?;
                f.write_str("@")?;
                f.write_str(loc)?;
                f.write_str("] ")?;
            }
        }

        Ok(())
    }
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Key(Arc<KeyInner>);
impl Debug for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}
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

type TreeStateInner = HashMap<Key, Box<dyn Any + Send + Sync>>;
type TreeState = Arc<RwLock<TreeStateInner>>;

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub key: Key,
    pub tree: TreeState,
    pub tree_next: TreeState,
    pub used: Arc<Mutex<HashSet<Key>>>,
    pub touched: Arc<Mutex<HashSet<Key>>>,
}
impl Context {
    pub fn sideband(&self, key: String) -> Self {
        Context {
            key: Key(Arc::new(KeyInner::Sideband { key })),
            tree: self.tree.clone(),
            tree_next: self.tree_next.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_state_value(&self, key: String) -> Self {
        Context {
            key: self.key.enter_state_value(key),
            tree: self.tree.clone(),
            tree_next: self.tree_next.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_widget(&self, name: &'static str, loc: &'static str) -> Self {
        Context {
            key: self.key.enter_widget(name, loc),
            tree: self.tree.clone(),
            tree_next: self.tree_next.clone(),
            used: Default::default(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_widget_key(&self, name: &'static str, loc: &'static str, key: String) -> Self {
        Context {
            key: self.key.enter_widget_key(name, loc, key),
            tree: self.tree.clone(),
            tree_next: self.tree_next.clone(),
            used: Default::default(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_hook(&self, name: &'static str, loc: &'static str) -> Self {
        Context {
            key: self.key.enter_hook(name, loc),
            tree: self.tree.clone(),
            tree_next: self.tree_next.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }
    pub fn with_key(&self, key: Key) -> Context {
        Context {
            key,
            tree: self.tree.clone(),
            tree_next: self.tree_next.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }

    pub fn mark_used(&self, key: &Key) {
        self.used.lock().insert(key.clone());
    }
    pub fn touch(&self) {
        self.touched.lock().insert(self.key.clone());
    }
    pub fn update_tree(&mut self) {
        let mut touched = self.touched.lock();
        let mut tree_next = self.tree_next.write();
        let mut tree = self.tree.write();
        touched.drain();
        for (key, value) in tree_next.drain() {
            touched.insert(key.clone());
            tree.insert(key, value);
        }
    }

    pub fn ident(&self) -> *const Box<dyn Any + Send + Sync> {
        &self.tree.read()[&self.key] as *const Box<dyn Any + Send + Sync>
    }
    pub fn is_present(&self) -> bool { self.tree.read().contains_key(&self.key) }
}
impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        (self.key == other.key) && Arc::ptr_eq(&self.tree, &other.tree)
    }
}

#[derive(Clone)]
pub struct StateValue<T> {
    pub context: Context,
    phantom: PhantomData<T>,
}
impl<T: 'static + Sync + Send + Debug> Debug for StateValue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateValue")
            .field("key", &self.context.key)
            .field("value", &*self.get_ref_sneaky())
            .finish()
    }
}
impl<T> StateValue<T> {
    pub fn new(context: Context, key: &str) -> Self {
        StateValue {
            context: context.enter_state_value(key.to_string()),
            phantom: PhantomData::default(),
        }
    }
}
impl<T> PartialEq for StateValue<T> {
    fn eq(&self, other: &Self) -> bool { self.context == other.context }
}
impl<T: 'static + Sync + Send> StateValue<T> {
    pub fn set(&self, new_value: T) {
        self.context.tree_next.write().insert(self.context.key.clone(), Box::new(new_value));
    }
    pub fn set_now(&self, new_value: T) {
        self.context.touch();
        self.set_sneaky_now(new_value)
    }
    pub fn set_sneaky_now(&self, new_value: T) {
        self.context.tree.write().insert(self.context.key.clone(), Box::new(new_value));
    }
    pub fn get_ref_sneaky(&self) -> StateValueGuard<T> {
        StateValueGuard {
            rw_lock_guard: self.context.tree.read(),
            path: self.context.key.clone(),
            phantom: Default::default(),
        }
    }
}
impl<T: 'static + Sync + Send + PartialEq> StateValue<T> {
    pub fn update(&self, new_value: T) {
        if !self.context.is_present() || &*self.get_ref_sneaky() != &new_value {
            self.set(new_value)
        }
    }
    pub fn update_now(&self, new_value: T) {
        if !self.context.is_present() || &*self.get_ref_sneaky() != &new_value {
            self.set_now(new_value)
        }
    }
}
impl<T: Clone + 'static> StateValue<T> {
    pub fn get_sneaky(&self) -> T {
        if !self.context.is_present() {
            panic!("no entry found for key {:?}", self.context.key.clone());
        }
        self.context.tree.read()[&self.context.key].downcast_ref::<T>().unwrap().clone()
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
