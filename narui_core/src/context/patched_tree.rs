use super::key::KeyMap;
use crate::{Fragment, Key};
use dashmap::DashMap;
use freelist::{FreeList, Idx};
use hashbrown::HashMap;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::{any::Any, num::NonZeroUsize, ops::Deref};

type Dependents = tinyset::Set64<usize>;
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
    data: RwLock<FreeList<(Dependents, TreeItem)>>,
    key_to_idx: RwLock<HashMap<Key, HashMap<u16, Idx>>>,
    patch: FxDashMap<Idx, Patch<TreeItem>>,
}

type DataRef<'a> = MappedRwLockReadGuard<'a, (Dependents, TreeItem)>;
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
                Some(v) => &(*v).1,
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

    pub fn remove_patch(&self, idx: HookRef) { self.patch.remove(&idx.1); }

    pub fn initialize(&self, key: HookKey, value: TreeItem) -> HookRef {
        (
            key,
            *self
                .key_to_idx
                .write()
                .entry(key.0)
                .or_default()
                .entry(key.1)
                .or_insert_with(|| self.data.write().add((Default::default(), value))),
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
                .or_insert_with(|| self.data.write().add((Default::default(), gen()))),
        )
    }

    pub fn set(&self, idx: HookRef, value: TreeItem) {
        self.patch.insert(idx.1, Patch { value, key: idx.0 });
    }

    pub fn set_unconditional(&self, idx: Idx, value: TreeItem) { self.data.write()[idx].1 = value; }

    pub fn remove_widget(&self, key: &Key) {
        if let Some(indices) = self.key_to_idx.write().remove(key) {
            for idx in indices.values() {
                self.data.write().remove(*idx);
            }
        }
    }

    // apply the patch to the tree starting a new frame
    pub fn update_tree<'a>(&'a self, _key_map: &mut KeyMap) -> impl Iterator<Item = HookRef> + 'a {
        let mut keys = vec![];
        for kv in self.patch.iter() {
            keys.push(*kv.key());
        }

        keys.into_iter().map(move |idx| {
            let (idx, Patch { value, key }) = self.patch.remove(&idx).unwrap();
            self.set_unconditional(idx, value);

            (key, idx)
        })
    }

    pub fn set_dependent(&self, key: HookRef, frag: Fragment) {
        self.data.write()[key.1].0.insert(frag.0.get());
    }

    pub fn dependents(&'_ self, key: HookRef) -> impl Iterator<Item = Fragment> + '_ {
        std::mem::take(&mut self.data.write()[key.1].0)
            .into_iter()
            .map(|v| Fragment(unsafe { NonZeroUsize::new_unchecked(v) }))
    }
}
