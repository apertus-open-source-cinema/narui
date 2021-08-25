use crate::{Key, KeyPart, LayoutTree, Layouter, PositionedRenderObject};
use dashmap::DashMap;
use derivative::Derivative;
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
    map: HashMap<Key, Vec<Box<dyn Any>>, ahash::RandomState>,
    dirty: HashSet<Key, ahash::RandomState>,
}

impl ArgsTree {
    pub fn set(&mut self, key: Key, values: Vec<Box<dyn Any>>) {
        self.dirty.insert(key);
        self.map.insert(key, values);
    }

    pub fn get(&self, key: &Key) -> Option<&Vec<Box<dyn Any>>> { self.map.get(key) }

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
    #[derivative(Debug(format_with = "crate::util::format_helpers::print_vec_len"))]
    pub(crate) after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
}

impl<'a> WidgetContext<'a> {
    pub fn key_for_hook(&mut self) -> Key {
        let counter = self.widget_local.hook_counter;
        self.widget_local.hook_counter += 1;
        self.widget_local.key.with(KeyPart::Hook(counter))
    }

    pub fn thread_context(&self) -> ThreadContext { ThreadContext { tree: self.tree.clone() } }

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

pub type TreeItem = Box<dyn Any + Send + Sync>;

#[derive(Debug)]
enum Patch<T> {
    Remove,
    Set(T),
}

// TODO(robin): investigate evmap instead
type FxDashMap<K, V> = DashMap<K, V, ahash::RandomState>;

#[derive(Debug, Default)]
pub struct PatchedTree {
    tree: FxDashMap<Key, TreeItem>,
    patch: FxDashMap<Key, Patch<TreeItem>>,
}

type HashRef<'a> = dashmap::mapref::one::Ref<'a, Key, TreeItem, ahash::RandomState>;
type HashPatchRef<'a> = dashmap::mapref::one::Ref<'a, Key, Patch<TreeItem>, ahash::RandomState>;

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

    pub fn set(&self, key: Key, value: TreeItem) { self.patch.insert(key, Patch::Set(value)); }
    pub fn set_unconditional(&self, key: Key, value: TreeItem) { self.tree.insert(key, value); }
    pub fn remove(&self, key: Key) { self.patch.insert(key, Patch::Remove); }

    // apply the patch to the tree starting a new frame
    pub fn update_tree(&self) -> Vec<Key> {
        let mut keys = vec![];
        for kv in self.patch.iter() {
            keys.push(kv.key().clone());
        }

        for key in &keys {
            let (key, value) = self.patch.remove(key).unwrap();
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
    pub hook_counter: u16,
    pub used: HashSet<Key, ahash::RandomState>,
}

impl WidgetLocalContext {
    pub fn mark_used(&mut self, key: Key) { self.used.insert(key); }

    pub fn for_key(key: Key) -> Self { Self { key, ..Default::default() } }
}
