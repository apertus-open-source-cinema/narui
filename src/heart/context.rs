use hashbrown::{HashMap, HashSet};
use parking_lot::{Mutex, RwLock};
use std::{
    any::Any,
    collections::hash_map::DefaultHasher,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    mem::MaybeUninit,
    sync::Arc,
};

#[derive(Eq, Copy, PartialOrd)]
pub struct Key {
    data: [KeyPart; 32],
    len: usize,
    hash: u64,
}
impl Key {
    pub fn with(&self, tail: KeyPart) -> Self {
        if self.len() == 31 {
            dbg!(&self);
            panic!("crank up the key length limit!");
        }
        let mut new = *self;
        new.data[self.len()] = tail;
        new.hash = self.hash.overflowing_add(KeyPart::calculate_hash(&tail)).0;
        new.len += 1;
        new
    }
    pub fn len(&self) -> usize { self.len }
    pub fn last_part(&self) -> KeyPart { self.data[self.len() - 1] }
    pub fn starts_with(&self, start: &Key) -> bool {
        for i in 0..(start.len()) {
            if self.data[i] != start.data[i] {
                return false;
            }
        }
        return true;
    }
}
impl Default for Key {
    fn default() -> Self {
        let data = unsafe { MaybeUninit::uninit().assume_init() };
        Self { data, len: 0, hash: 0 }
    }
}
impl Clone for Key {
    fn clone(&self) -> Self {
        let data = unsafe {
            let mut uninit: [KeyPart; 32] = MaybeUninit::uninit().assume_init();
            uninit[..self.len()].clone_from_slice(&self.data[..self.len()]);
            uninit
        };
        Key { data, len: self.len, hash: self.hash }
    }
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        for i in (0..self.len).rev() {
            if self.data[i] != other.data[i] {
                return false;
            }
        }
        true
    }
}
impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) { self.hash.hash(state) }
}
impl Debug for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.len() {
            write!(f, "{}{:?}", if i == 0 { "" } else { "." }, self.data[i])?
        }
        Ok(())
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub enum KeyPart {
    Nop,
    DebugLayoutBounds,
    Widget,
    Deps,

    Arg(u64),
    Hook(u64),
    RenderObject(u64),

    Sideband { hash: u64 },

    Fragment { name: &'static str, loc: &'static str },
    FragmentKey { name: &'static str, loc: &'static str, hash: u64 },
}
impl KeyPart {
    pub fn calculate_hash<T: Hash + ?Sized>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    pub fn sideband<T: Hash + ?Sized>(t: &T) -> Self {
        Self::Sideband { hash: Self::calculate_hash(t) }
    }
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
            KeyPart::Deps => write!(f, "Deps"),
            KeyPart::Sideband { hash } => write!(f, "Sideband_{}", hash),
            KeyPart::Hook(number) => write!(f, "Sideband_{}", number),
            KeyPart::RenderObject(number) => write!(f, "RenderObject_{}", number),
            KeyPart::Fragment { name, loc } => write!(f, "Fragment_{}_{}", name, loc),
            KeyPart::FragmentKey { name, loc, hash } => {
                write!(f, "Fragment_{}_{}_{}", name, loc, hash)
            }
        }
    }
}

pub type TreeItem = Box<dyn Any + Send + Sync>;

#[derive(Debug)]
enum Patch<T> {
    Remove,
    Set(T),
}

#[derive(Debug, Default)]
pub struct PatchedTree {
    tree: HashMap<Key, TreeItem>,
    patch: HashMap<Key, Patch<TreeItem>>,
}
impl PatchedTree {
    pub fn get(&self, key: Key) -> Option<&TreeItem> {
        match self.patch.get(&key) {
            None => self.tree.get(&key),
            Some(Patch::Remove) => None,
            Some(Patch::Set(v)) => Some(v),
        }
    }

    pub fn set(&mut self, key: Key, value: TreeItem) { self.patch.insert(key, Patch::Set(value)); }
    pub fn remove(&mut self, key: Key) { self.patch.insert(key, Patch::Remove); }

    // apply the patch to the tree starting a new frame
    pub fn update_tree(&mut self) -> Vec<Key> {
        let keys = self.patch.keys().into_iter().cloned().collect();
        for (key, value) in self.patch.drain() {
            match value {
                Patch::Remove => {
                    let keys: Vec<Key> = self.tree.keys().into_iter().cloned().collect();
                    for candidate in keys {
                        if candidate.starts_with(&key) {
                            self.tree.remove(&candidate);
                        }
                    }
                }
                Patch::Set(v) => {
                    self.tree.insert(key, v);
                }
            }
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
        self.widget_local.key.with(KeyPart::Hook(to_return))
    }
}
