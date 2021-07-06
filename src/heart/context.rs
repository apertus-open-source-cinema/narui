use hashbrown::{HashMap, HashSet};
use parking_lot::{Mutex, RwLock};
use std::{any::Any, collections::hash_map::DefaultHasher, fmt::Debug, hash::{Hash, Hasher}, sync::Arc};
use std::fmt::Formatter;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct Key([Option<KeyPart>; 32]);
impl Default for Key {
    fn default() -> Self { Self([None; 32]) }
}
impl Key {
    pub fn with(&self, tail: KeyPart) -> Self {
        let mut to_return = self.0;
        for i in 0..(to_return.len() + 1) {
            if i == to_return.len() {
                dbg!(&self);
                panic!("crank up the key length limit!");
            }
            if to_return[i].is_none() {
                to_return[i] = Some(tail);
                break;
            }
        }
        Self(to_return)
    }
    pub fn len(&self) -> usize {
        for i in 0..(self.0.len()) {
            if self.0[i].is_none() {
                return i;
            }
        }
        return self.0.len();
    }
    pub fn last_part(&self) -> KeyPart {
        self.0[self.len() - 1].unwrap()
    }
}
impl Debug for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.len() {
            write!(f, "{}{:?}", if i == 0 {""} else {"."}, self.0[i].unwrap())?
        }
        Ok(())
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub enum KeyPart {
    Nop,
    DebugLayoutBounds,
    Widget,
    Sideband { hash: u64 },

    Arg(usize),
    Hook { number: u64 },
    RenderObject { number: u64 },

    Fragment { name: &'static str, loc: &'static str },
    FragmentKey { name: &'static str, loc: &'static str, hash: u64 },
}
impl KeyPart {
    pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    pub fn sideband<T: Hash>(t: &T) -> Self { Self::Sideband { hash: Self::calculate_hash(t) } }
}
impl Default for KeyPart {
    fn default() -> Self { KeyPart::Nop }
}
impl Debug for KeyPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyPart::Nop => write!(f, "Nop"),
            KeyPart::Arg(n) => write!(f, "Arg_{}", n),
            KeyPart::DebugLayoutBounds => write!(f, "DebugLayoutBounds"),
            KeyPart::Widget => write!(f, "Widget"),
            KeyPart::Sideband { hash } => write!(f, "Sideband_{}", hash),
            KeyPart::Hook { number } => write!(f, "Sideband_{}", number),
            KeyPart::RenderObject { number } => write!(f, "RenderObject_{}", number),
            KeyPart::Fragment { name, loc } => write!(f, "Fragment_{}_{}", name, loc),
            KeyPart::FragmentKey { name, loc, hash } => write!(f, "Fragment_{}_{}_{}", name, loc, hash)
        }
    }
}

pub type TreeItem = Box<dyn Any + Send + Sync>;
pub type TreeStateInner = HashMap<Key, TreeItem>;

#[derive(Debug, Default)]
pub struct PatchedTree {
    tree: TreeStateInner,
    patch: TreeStateInner,
}
impl PatchedTree {
    pub fn get(&self, key: Key) -> Option<&TreeItem> {
        self.patch.get(&key).or_else(|| self.tree.get(&key))
    }

    pub fn set(&mut self, key: Key, value: TreeItem) { self.patch.insert(key, value); }

    pub fn is_updated(&self, key: Key, is_equal: impl Fn(&TreeItem, &TreeItem) -> bool) -> bool {
        match (self.tree.get(&key), self.patch.get(&key)) {
            (None, None) => false,
            (Some(_), None) => false,
            (None, Some(_)) => true,
            (Some(a), Some(b)) => !is_equal(a, b),
        }
    }

    // apply the patch to the tree starting a new frame
    pub fn update_tree(&mut self) -> Vec<Key> {
        let keys = self.patch.keys().into_iter().cloned().collect();
        for (key, value) in self.patch.drain() {
            self.tree.insert(key, value);
        }
        keys
    }
}

#[derive(Clone, Debug, Default)]
pub struct WidgetLocalContext {
    pub key: Key,
    pub hook_counter: Arc<Mutex<u64>>,
    pub used: Arc<Mutex<HashSet<Key>>>,
}
impl WidgetLocalContext {
    pub fn mark_used(&self, key: Key) { self.used.lock().insert(key); }
}

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub global: Arc<RwLock<PatchedTree>>,
    pub widget_local: WidgetLocalContext,
}
impl Context {
    pub fn with_key_widget(&self, key: Key) -> Context {
        Context {
            global: self.global.clone(),
            widget_local: WidgetLocalContext {
                key,
                hook_counter: Default::default(),
                used: Default::default(),
            },
        }
    }

    pub fn key_for_hook(&self) -> Key {
        let mut counter = self.widget_local.hook_counter.lock();
        let to_return = *counter;
        *counter += 1;
        self.widget_local.key.with(KeyPart::Hook { number: to_return })
    }
}
