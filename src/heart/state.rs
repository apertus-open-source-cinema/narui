use hashbrown::{HashMap, HashSet};
use std::{
    any::Any,
    fmt,
    fmt::{Debug, Formatter},
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
};

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
    pub used: Arc<Mutex<HashSet<Key>>>,
    pub touched: Arc<Mutex<HashSet<Key>>>,
}
impl Context {
    pub fn sideband(&self, key: String) -> Self {
        Context {
            key: Key(Arc::new(KeyInner::Sideband { key })),
            tree: self.tree.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_state_value(&self, key: String) -> Self {
        Context {
            key: self.key.enter_state_value(key),
            tree: self.tree.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_widget(&self, name: &'static str, loc: &'static str) -> Self {
        Context {
            key: self.key.enter_widget(name, loc),
            tree: self.tree.clone(),
            used: Default::default(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_widget_key(&self, name: &'static str, loc: &'static str, key: String) -> Self {
        Context {
            key: self.key.enter_widget_key(name, loc, key),
            tree: self.tree.clone(),
            used: Default::default(),
            touched: self.touched.clone(),
        }
    }
    pub fn enter_hook(&self, name: &'static str, loc: &'static str) -> Self {
        Context {
            key: self.key.enter_hook(name, loc),
            tree: self.tree.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }
    pub fn mark_used(&self) { self.used.lock().unwrap().insert(self.key.clone()); }
    pub fn touch(&self) { self.touched.lock().unwrap().insert(self.key.clone()); }
    pub fn finish_touched(&mut self) -> Arc<Mutex<HashSet<Key>>> {
        let old_touched = self.touched.clone();
        self.touched = Default::default();
        old_touched
    }

    pub fn with_key(&self, key: Key) -> Context {
        Context {
            key,
            tree: self.tree.clone(),
            used: self.used.clone(),
            touched: self.touched.clone(),
        }
    }

    pub fn ident(&self) -> *const Box<dyn Any + Send + Sync> {
        &self.tree.read().unwrap()[&self.key] as *const Box<dyn Any + Send + Sync>
    }
    pub fn is_present(&self) -> bool { self.tree.read().unwrap().contains_key(&self.key) }
}
impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        (self.key == other.key) && Arc::ptr_eq(&self.tree, &other.tree)
    }
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
impl<T> PartialEq for StateValue<T> {
    fn eq(&self, other: &Self) -> bool { self.context == other.context }
}
impl<T: 'static + Sync + Send> StateValue<T> {
    pub fn set(&self, new_value: T) {
        self.context.touch();
        self.set_sneaky(new_value)
    }
    pub fn set_sneaky(&self, new_value: T) {
        self.context.tree.write().unwrap().insert(self.context.key.clone(), Box::new(new_value));
    }
    pub fn get_ref(&self) -> StateValueGuard<T> {
        self.context.mark_used();
        StateValueGuard {
            rw_lock_guard: self.context.tree.read().unwrap(),
            path: self.context.key.clone(),
            phantom: Default::default(),
        }
    }
}
impl<T: Clone + 'static> StateValue<T> {
    pub fn get(&self) -> T {
        self.context.mark_used();
        self.context.tree.read().unwrap()[&self.context.key].downcast_ref::<T>().unwrap().clone()
    }

    pub fn get_default(&self, default: T) -> T {
        self.context.mark_used();
        match self.context.tree.read().unwrap().get(&self.context.key) {
            Some(v) => v.downcast_ref::<T>().unwrap().clone(),
            None => default,
        }
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
