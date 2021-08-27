use crate::{EvaluatedFragment, Key, KeyMap, Layouter};
use dashmap::DashMap;
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};

use freelist::FreeList;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use rutter_layout::Idx;
use std::{any::Any, cell::RefCell, fmt::Debug, ops::Deref, rc::Rc, sync::Arc};

#[derive(Debug, Default)]
pub struct ArgsTree {
    data: FreeList<Vec<Box<dyn Any>>>,
    map: HashMap<Key, Idx>,
    dirty: HashSet<Key, ahash::RandomState>,
}

impl ArgsTree {
    pub fn set(&mut self, key: Key, values: Vec<Box<dyn Any>>) -> Idx {
        self.dirty.insert(key);
        match self.map.get(&key) {
            Some(idx) => {
                self.data[*idx] = values;
                *idx
            }
            None => {
                let idx = self.data.add(values);
                self.map.insert(key, idx);
                idx
            }
        }
    }
    pub fn add(&mut self, key: Key, values: Vec<Box<dyn Any>>) -> Idx {
        self.dirty.insert(key);
        let idx = self.data.add(values);
        self.map.insert(key, idx);
        idx
    }

    pub fn set_unconditional(&mut self, key: Key, idx: Idx, values: Vec<Box<dyn Any>>) -> Idx {
        self.dirty.insert(key);
        self.data[idx] = values;
        idx
    }

    pub fn get_idx(&self, key: Key) -> Option<Idx> { self.map.get(&key).cloned() }

    pub fn get_unconditional(&self, idx: Idx) -> &Vec<Box<dyn Any>> { &self.data[idx] }

    pub fn remove(&mut self, key_map: &mut KeyMap, root: Key) {
        log::trace!("removing {:?}", key_map.key_debug(root));
        if let Some(idx) = self.map.remove(&root) {
            self.data.remove(idx);
        }
    }

    pub fn dirty<'a>(&'a mut self) -> impl Iterator<Item = Key> + 'a { self.dirty.drain() }

    pub fn debug_print(&self, key_map: &KeyMap) -> String {
        let mut res = "ArgsTree {\n  map: {\n".to_string();
        for (key, args) in &self.map {
            res += &format!("    {:?}: {:?},\n", key_map.key_debug(*key), args);
        }
        res += "  },\n  dirty: {\n";

        for key in &self.dirty {
            res += &format!("    {:?},\n", key_map.key_debug(*key));
        }
        res += "}\n";

        res
    }
}

#[derive(Debug, Default)]
pub struct ExternalHookCount {
    counts: HashMap<Key, u16>,
}

impl ExternalHookCount {
    fn next(&mut self, key: Key) -> u16 {
        let count = self.counts.entry(key).or_insert(0);
        let idx = *count;
        *count += 1;
        idx
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WidgetContext<'a> {
    pub widget_local: WidgetLocalContext,
    #[derivative(Debug = "ignore")]
    pub tree: Arc<PatchedTree>,
    pub local_hook: bool,
    pub external_hook_count: &'a mut ExternalHookCount,
    pub args_tree: &'a mut ArgsTree,
    pub widget_loc: (usize, usize),
    #[derivative(Debug(format_with = "crate::util::format_helpers::print_vec_len"))]
    pub(crate) after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
    pub key_map: &'a mut KeyMap,
}

impl<'a> WidgetContext<'a> {
    pub fn key_for_hook(&mut self) -> HookKey {
        if self.local_hook {
            let counter = self.widget_local.hook_counter;
            self.widget_local.hook_counter += 1;
            log::trace!(
                "creating local hook: {:?}:{}",
                self.key_map.key_debug(self.widget_local.key),
                counter
            );
            (self.widget_local.key, counter)
        } else {
            let key = self.external_hook_count.next(self.widget_local.key) | 0b1000_0000_0000_0000;
            log::trace!(
                "creating external hook: {:?}:{}",
                self.key_map.key_debug(self.widget_local.key),
                key
            );
            (self.widget_local.key, key)
        }
    }

    pub fn thread_context(&self) -> ThreadContext { ThreadContext { tree: self.tree.clone() } }

    pub fn root(
        tree: Arc<PatchedTree>,
        external_hook_count: &'a mut ExternalHookCount,
        args_tree: &'a mut ArgsTree,
        after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
        key_map: &'a mut KeyMap,
    ) -> Self {
        Self {
            tree,
            after_frame_callbacks,
            args_tree,
            widget_local: Default::default(),
            widget_loc: (0, 0),
            key_map,
            external_hook_count,
            local_hook: true,
        }
    }

    pub fn for_fragment(
        tree: Arc<PatchedTree>,
        external_hook_count: &'a mut ExternalHookCount,
        args_tree: &'a mut ArgsTree,
        key: Key,
        after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
        key_map: &'a mut KeyMap,
    ) -> Self {
        WidgetContext {
            tree,
            after_frame_callbacks,
            args_tree,
            widget_local: WidgetLocalContext::for_key(key),
            widget_loc: (0, 0),
            key_map,
            external_hook_count,
            local_hook: true,
        }
    }

    pub fn with_key_widget(&mut self, key: Key) -> WidgetContext {
        WidgetContext {
            tree: self.tree.clone(),
            local_hook: true,
            external_hook_count: &mut self.external_hook_count,
            args_tree: self.args_tree,
            widget_loc: (0, 0),
            after_frame_callbacks: self.after_frame_callbacks,
            widget_local: WidgetLocalContext::for_key(key),
            key_map: &mut self.key_map,
        }
    }
}

#[derive(Clone)]
pub struct ThreadContext {
    pub(crate) tree: Arc<PatchedTree>,
}

pub struct CallbackContext<'a> {
    pub(crate) tree: Arc<PatchedTree>,
    pub key_map: &'a KeyMap,
    pub(crate) layout: &'a Layouter,
    pub(crate) key_to_fragment: &'a HashMap<Key, Rc<RefCell<EvaluatedFragment>>>,
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
struct Patch<T> {
    key: HookKey,
    value: T,
}

// TODO(robin): investigate evmap instead
type FxDashMap<K, V> = DashMap<K, V, ahash::RandomState>;

// 15 bits of idx + top bit set if external
pub type HookKey = (Key, u16);
pub type HookRef = (HookKey, Idx);

#[derive(Debug, Default)]
pub struct PatchedTree {
    data: RwLock<FreeList<TreeItem>>,
    key_to_idx: RwLock<HashMap<Key, HashMap<u16, Idx>>>,
    patch: FxDashMap<Idx, Patch<TreeItem>>,
}

type DataRef<'a> = MappedRwLockReadGuard<'a, Box<dyn Any + Send + Sync>>;
type HashPatchRef<'a> = dashmap::mapref::one::Ref<'a, Idx, Patch<TreeItem>, ahash::RandomState>;

pub struct PatchTreeEntry<'a> {
    patched_entry: Option<HashPatchRef<'a>>,
    unpatched_entry: Option<DataRef<'a>>,
}

impl<'a> PatchTreeEntry<'a> {
    fn new(patched_entry: Option<HashPatchRef<'a>>, unpatched_entry: Option<DataRef<'a>>) -> Self {
        Self { patched_entry, unpatched_entry }
    }
}

impl<'a> Deref for PatchTreeEntry<'a> {
    type Target = TreeItem;

    fn deref(&self) -> &Self::Target {
        match &self.patched_entry {
            Some(p) => &p.value().value,
            None => match &self.unpatched_entry {
                Some(v) => &*v,
                None => unreachable!(),
            },
        }
    }
}

impl PatchedTree {
    pub fn get_patched(&self, idx: HookRef) -> PatchTreeEntry {
        match self.patch.get(&idx.1) {
            None => self.get_unpatched(idx),
            Some(patch) => PatchTreeEntry::new(Some(patch), None),
        }
    }

    pub fn get_unpatched(&self, idx: HookRef) -> PatchTreeEntry {
        PatchTreeEntry::new(None, Some(RwLockReadGuard::map(self.data.read(), |v| &v[idx.1])))
    }

    pub fn initialize(&self, key: HookKey, value: TreeItem) -> HookRef {
        (
            key,
            *self
                .key_to_idx
                .write()
                .entry(key.0)
                .or_default()
                .entry(key.1)
                .or_insert_with(|| self.data.write().add(value)),
        )
    }

    pub fn initialize_with(&self, key: HookKey, gen: impl FnOnce() -> TreeItem) -> HookRef {
        (
            key,
            *self
                .key_to_idx
                .write()
                .entry(key.0)
                .or_default()
                .entry(key.1)
                .or_insert_with(|| self.data.write().add(gen())),
        )
    }

    pub fn set(&self, idx: HookRef, value: TreeItem) {
        self.patch.insert(idx.1, Patch { value, key: idx.0 });
    }

    pub fn set_unconditional(&self, idx: Idx, value: TreeItem) { self.data.write()[idx] = value; }

    pub fn remove_widget(&self, key: &Key) {
        if let Some(indices) = self.key_to_idx.write().remove(key) {
            for idx in indices.values() {
                self.data.write().remove(*idx);
            }
        }
    }

    // apply the patch to the tree starting a new frame
    pub fn update_tree<'a>(&'a self, _key_map: &mut KeyMap) -> impl Iterator<Item = HookKey> + 'a {
        let mut keys = vec![];
        for kv in self.patch.iter() {
            keys.push(*kv.key());
        }

        keys.into_iter().map(move |idx| {
            let (idx, Patch { value, key }) = self.patch.remove(&idx).unwrap();
            self.set_unconditional(idx, value);

            key
        })
    }
}

pub type AfterFrameCallback = Box<dyn for<'a> Fn(&'a CallbackContext<'a>)>;

#[derive(Clone, Debug, Default)]
pub struct WidgetLocalContext {
    pub key: Key,
    pub hook_counter: u16,
    pub used: HashSet<HookKey, ahash::RandomState>,
}

impl WidgetLocalContext {
    pub fn mark_used(&mut self, key: HookKey) { self.used.insert(key); }

    pub fn for_key(key: Key) -> Self { Self { key, ..Default::default() } }
}
