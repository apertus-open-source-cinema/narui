use crate::{LayoutTree, Layouter, PositionedRenderObject};
use dashmap::DashMap;
use derivative::Derivative;
use fxhash::FxBuildHasher;
use hashbrown::{HashMap, HashSet};
use parking_lot::{Mutex, RwLock};
use std::{
    any::Any,
    collections::hash_map::DefaultHasher,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    mem::MaybeUninit,
    ops::Deref,
    rc::Rc,
    sync::Arc,
};

#[derive(Debug, Default)]
pub struct ArgsTree {
    map: HashMap<Key, Box<dyn Any>>,
    dirty: HashSet<Key>,
}

impl ArgsTree {
    pub fn set(&mut self, key: Key, value: Box<dyn Any>) {
        self.dirty.insert(key.parent().clone());
        self.map.insert(key.clone(), value);
    }

    pub fn get(&self, key: &Key) -> Option<&Box<dyn Any>> { self.map.get(key) }

    pub fn remove(&mut self, root: Key) { self.map.retain(|k, v| !k.starts_with(&root)); }

    pub fn dirty<'a>(&'a mut self) -> impl Iterator<Item = Key> + 'a { self.dirty.drain() }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WidgetContext<'a> {
    pub widget_local: WidgetLocalContext,
    #[derivative(Debug = "ignore")]
    pub tree: Arc<PatchedTree>,
    pub args_tree: &'a mut ArgsTree,
    // TODO(robin): is there any way to pass a &mut ref to WidgetContext around?
    #[derivative(Debug(format_with = "crate::util::format_helpers::print_vec_len"))]
    pub(crate) after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
}

impl<'a> WidgetContext<'a> {
    pub fn key_for_hook(&mut self) -> Key {
        let counter = self.widget_local.hook_counter;
        self.widget_local.hook_counter += 1;
        self.widget_local.key.with(KeyPart::Hook(counter))
    }

    pub fn thread_context(&self) -> ThreadContext {
        ThreadContext {
            tree: self.tree.clone()
        }
    }

    pub fn root(
        tree: Arc<PatchedTree>,
        args_tree: &'a mut ArgsTree,
        after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
    ) -> Self {
        Self { tree, after_frame_callbacks, args_tree, widget_local: Default::default() }
    }

    pub fn for_fragment(
        tree: Arc<PatchedTree>,
        args_tree: &'a mut ArgsTree,
        key: Key,
        after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
    ) -> Self {
        WidgetContext {
            tree,
            after_frame_callbacks,
            args_tree,
            widget_local: WidgetLocalContext::for_key(key),
        }
    }

    pub fn with_key_widget<'cb>(&'cb mut self, key: Key) -> WidgetContext<'cb> {
        WidgetContext {
            tree: self.tree.clone(),
            args_tree: self.args_tree,
            after_frame_callbacks: self.after_frame_callbacks,
            widget_local: WidgetLocalContext::for_key(key),
        }
    }
}

#[derive(Clone)]
pub struct ThreadContext {
    pub(crate) tree: Arc<PatchedTree>,
}

pub struct CallbackContext<'a> {
    pub(crate) tree: Arc<PatchedTree>,
    pub(crate) layout: &'a Layouter,
}

// thread access
//   - get value (not listen because we don't have the rebuild if changed thing)
//   - shout
// widget access
//   - create listenable
//   - listen
//   - create after-frame-callback
// callback access
//   - shout
//   - get value
//   - measure

#[derive(Eq, Copy, PartialOrd)]
pub struct Key {
    data: [KeyPart; 32],
    len: usize,
    hash: u64,
}
impl Key {
    pub fn with(&self, tail: KeyPart) -> Self {
        if self.len() == 31 {
            panic!("crank up the key length limit!");
        }
        let mut new = *self;
        new.data[self.len()] = tail;
        new.hash = self.hash.overflowing_add(KeyPart::calculate_hash(&tail)).0;
        new.len += 1;
        new
    }
    pub fn parent(&self) -> Self {
        let mut new = *self;
        let tail = new.data[self.len() - 1];
        new.hash = self.hash.overflowing_sub(KeyPart::calculate_hash(&tail)).0;
        new.len -= 1;
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
        true
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
        write!(f, "RootKey")?;
        for i in 0..self.len() {
            write!(f, ".{:?}", self.data[i])?
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

    Arg(&'static str),
    Hook(u64),
    RenderObject(u64),
    Rsx(u64),

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
            KeyPart::Arg(s) => write!(f, "Arg_{}", s),
            KeyPart::DebugLayoutBounds => write!(f, "DebugLayoutBounds"),
            KeyPart::Widget => write!(f, "Widget"),
            KeyPart::Deps => write!(f, "Deps"),
            KeyPart::Sideband { hash } => write!(f, "Sideband_{}", hash),
            KeyPart::Hook(number) => write!(f, "Hook_{}", number),
            KeyPart::RenderObject(number) => write!(f, "RenderObject_{}", number),
            KeyPart::Fragment { name, loc } => write!(f, "Fragment_{}_{}", name, loc),
            KeyPart::FragmentKey { name, loc, hash } => {
                write!(f, "Fragment_{}_{}_{}", name, loc, hash)
            }
            KeyPart::Rsx(hash) => write!(f, "Rsx_{}", hash),
        }
    }
}

pub type TreeItem = Box<dyn Any + Send + Sync>;

#[derive(Debug)]
enum Patch<T> {
    Remove,
    Set(T),
}

// TODO(robin): investigate evmap instead
type FxDashMap<K, V> = DashMap<K, V, FxBuildHasher>;

#[derive(Debug, Default)]
pub struct PatchedTree {
    tree: FxDashMap<Key, TreeItem>,
    patch: FxDashMap<Key, Patch<TreeItem>>,
}

type HashRef<'a> = dashmap::mapref::one::Ref<'a, Key, TreeItem, FxBuildHasher>;
type HashPatchRef<'a> = dashmap::mapref::one::Ref<'a, Key, Patch<TreeItem>, FxBuildHasher>;

pub struct PatchTreeEntry<'a> {
    patched_entry: Option<HashPatchRef<'a>>,
    unpatched_entry: Option<HashRef<'a>>,
}

impl<'a> PatchTreeEntry<'a> {
    fn new(patched_entry: Option<HashPatchRef<'a>>, unpatched_entry: Option<HashRef<'a>>) -> Self {
        Self { patched_entry, unpatched_entry }
    }
}

impl<'a> Deref for PatchTreeEntry<'a> {
    type Target = TreeItem;

    fn deref(&self) -> &Self::Target {
        match &self.patched_entry {
            Some(p) => match p.value() {
                Patch::Remove => unreachable!(),
                Patch::Set(v) => v,
            },
            None => match &self.unpatched_entry {
                Some(v) => v.value(),
                None => unreachable!(),
            },
        }
    }
}

impl PatchedTree {
    pub fn get_patched(&self, key: &Key) -> Option<PatchTreeEntry> {
        dbg!("calling get_patched");
        match self.patch.get(key) {
            None => {
                if let Some(entry) = self.tree.get(key) {
                    Some(PatchTreeEntry::new(None, Some(entry)))
                } else {
                    None
                }
            }
            Some(patch) => match patch.value() {
                Patch::Remove => None,
                Patch::Set(_) => Some(PatchTreeEntry::new(Some(patch), None)),
            },
        }
    }

    pub fn get_unpatched(&self, key: &Key) -> Option<PatchTreeEntry> {
        if let Some(entry) = self.tree.get(key) {
            Some(PatchTreeEntry::new(None, Some(entry)))
        } else {
            None
        }
    }

    pub fn set(&self, key: Key, value: TreeItem) {
        dbg!("setting", key, &value);
        self.patch.insert(key, Patch::Set(value));
        dbg!("done with setting", key);
    }
    pub fn set_unconditional(&self, key: Key, value: TreeItem) { self.tree.insert(key, value); }
    pub fn remove(&self, key: Key) { self.patch.insert(key, Patch::Remove); }

    // apply the patch to the tree starting a new frame
    pub fn update_tree(&self) -> Vec<Key> {
        dbg!("updating tree");
        let mut keys = vec![];
        for kv in self.patch.iter() {
            keys.push(kv.key().clone());
        }

        for key in &keys {
            dbg!("removing", key);
            let (key, value) = self.patch.remove(key).unwrap();
            dbg!("done with removing", key, &value);
            match value {
                Patch::Remove => {
                    for candidate in self.tree.iter().map(|kv| kv.key().clone()) {
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

pub type AfterFrameCallback = Box<dyn for<'a> Fn(&'a CallbackContext<'a>)>;

#[derive(Clone, Debug, Default)]
pub struct WidgetLocalContext {
    pub key: Key,
    pub hook_counter: u64,
    pub used: HashSet<Key>,
}

impl WidgetLocalContext {
    pub fn mark_used(&mut self, key: Key) { self.used.insert(key); }

    pub fn for_key(key: Key) -> Self { Self { key, ..Default::default() } }
}
