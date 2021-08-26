use crate::{Key, KeyMap, KeyPart, Layouter};
use dashmap::DashMap;
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};

use std::{any::Any, fmt::Debug, ops::Deref, sync::Arc};
use crate::heart::owning_ref::OwningRef;

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

    pub fn remove(&mut self, key_map: &mut KeyMap, root: Key) {
        log::trace!("removing {:?}", key_map.key_debug(root));
        self.map.remove(&root);
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
    counts: HashMap<Key, u16>
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
            log::trace!("creating local hook: {:?}:{}", self.key_map.key_debug(self.widget_local.key), counter);
            (self.widget_local.key, counter)
        } else {
            let key = self.external_hook_count.next(self.widget_local.key) | 0b1000_0000_0000_0000;
            log::trace!("creating external hook: {:?}:{}", self.key_map.key_debug(self.widget_local.key), key);
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
            local_hook: true
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
            local_hook: true
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
struct Patch<T>(T);

// TODO(robin): investigate evmap instead
type FxDashMap<K, V> = DashMap<K, V, ahash::RandomState>;

// 15 bits of idx + top bit set if external
pub type HookKey = (Key, u16);

#[derive(Debug, Default)]
pub struct PatchedTree {
    tree: FxDashMap<Key, HashMap<u16, TreeItem>>,
    patch: FxDashMap<HookKey, Patch<TreeItem>>,
}

type HashRef<'a> = dashmap::mapref::one::Ref<'a, Key, HashMap<u16, TreeItem>, ahash::RandomState>;
type HashPatchRef<'a> = dashmap::mapref::one::Ref<'a, HookKey, Patch<TreeItem>, ahash::RandomState>;

pub struct PatchTreeEntry<'a> {
    patched_entry: Option<HashPatchRef<'a>>,
    unpatched_entry: Option<OwningRef<'a, HashRef<'a>, TreeItem>>,
}

impl<'a> PatchTreeEntry<'a> {
    fn new(patched_entry: Option<HashPatchRef<'a>>, unpatched_entry: Option<OwningRef<'a, HashRef<'a>, TreeItem>>) -> Self {
        Self { patched_entry, unpatched_entry }
    }
}

impl<'a> Deref for PatchTreeEntry<'a> {
    type Target = TreeItem;

    fn deref(&self) -> &Self::Target {
        match &self.patched_entry {
            Some(p) => &p.value().0,
            None => match &self.unpatched_entry {
                Some(v) => &*v,
                None => unreachable!(),
            },
        }
    }
}

impl PatchedTree {
    pub fn get_patched(&self, key: &HookKey) -> Option<PatchTreeEntry> {
        match self.patch.get(key) {
            None => self.get_unpatched(key),
            Some(patch) => {
                Some(PatchTreeEntry::new(Some(patch), None))
            },
        }
    }

    pub fn get_unpatched<'a>(&'a self, key: &HookKey) -> Option<PatchTreeEntry<'a>> {
        let key = *key;
        self.tree.get(&key.0).and_then(|entry| {
                match entry.get(&key.1) {
                    Some(_) => {
                        Some(PatchTreeEntry::new(None, Some(
                            unsafe {
                                OwningRef::new_assert_stable_address(entry).map_assert_stable_address(|v| {
                                    (*v).get(&key.1).unwrap()
                                })
                            }
                        )))
                    },
                    None => None
                }
        })
    }

    pub fn set(&self, key: HookKey, value: TreeItem) { self.patch.insert(key, Patch(value)); }
    pub fn set_unconditional(&self, key: HookKey, value: TreeItem) {
        self.tree.entry(key.0).or_default().insert(key.1, value);
    }
    pub fn remove_widget(&self, key: &Key) {
        self.tree.remove(key);
    }

    // apply the patch to the tree starting a new frame
    pub fn update_tree(&self, key_map: &mut KeyMap) -> impl Iterator<Item=HookKey> {
        let mut keys = vec![];
        for kv in self.patch.iter() {
            keys.push(*kv.key());
        }

        for key in &keys {
            let (key, Patch(v)) = self.patch.remove(key).unwrap();
            self.set_unconditional(key, v);
        }

        keys.into_iter()
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
