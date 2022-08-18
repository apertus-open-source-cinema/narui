use freelist::{FreeList, Idx};
use std::{
    fmt::{Debug, Formatter},
    hash::Hash,
};

type KeyInner = u32;
#[derive(Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub struct Key(pub(crate) KeyInner);

impl Default for Key {
    fn default() -> Self { Key::ROOT }
}

impl Key {
    const ROOT: Key = Key(1);
}

#[derive(Debug, Clone)]
struct KeyMapEntry {
    deleted: bool,
    tail: KeyPart,
    parent: KeyInner,
    child_idx: Idx,
    children_tails: Vec<i32>,
    children_keys: FreeList<KeyInner>,
    // last key that was accessed
    // especially for static widgets, the tails of the children should appear in order
    last: usize,
}

impl KeyMapEntry {
    fn deleted() -> Self {
        Self {
            deleted: true,
            tail: KeyPart::Root,
            parent: 1,
            child_idx: unsafe { Idx::new_unchecked(1) },
            children_tails: Default::default(),
            children_keys: Default::default(),
            last: 0,
        }
    }
}

#[derive(Debug)]
pub struct KeyMap {
    keys: Vec<KeyMapEntry>,
}

#[cfg(target_arch = "x86_64")]
fn find_avx2(data: &[i32], needle: i32) -> Option<usize> {
    use std::arch::x86_64::*;
    let trunc = (data.len() / 8) * 8;

    unsafe {
        let needle_wide = _mm256_set1_epi32(needle);
        for i in (0..trunc).step_by(8) {
            let addr = data.get_unchecked(i) as *const i32 as *const __m256i;
            let d = _mm256_loadu_si256(addr);
            let cmp = _mm256_cmpeq_epi32(needle_wide, d);
            let mask = _mm256_movemask_epi8(cmp);
            if mask != 0 {
                return Some(i + (mask.trailing_zeros() / 4) as usize);
            }
        }

        for i in trunc..data.len() {
            if *data.get_unchecked(i) == needle {
                return Some(i);
            }
        }
    }

    None
}

impl KeyMap {
    pub fn key_with(&mut self, parent: Key, tail: KeyPart, next: impl FnOnce() -> Key) -> Key {
        unsafe {
            let tail_packed = tail.pack();

            let idx = {
                let entry = self.keys.get_unchecked(parent.0 as usize);
                if (entry.last + 1) < entry.children_tails.len()
                    && *entry.children_tails.get_unchecked(entry.last + 1) == tail_packed
                {
                    Some(entry.last + 1)
                } else {
                    #[cfg(target_arch = "x86_64")]
                    {
                        find_avx2(&entry.children_tails[..], tail_packed)
                    }

                    #[cfg(not(target_arch = "x86_64"))]
                    {
                        entry.children_tails.iter().position(|v| *v == tail_packed)
                    }
                }
            };

            if let Some(idx) = idx {
                self.keys.get_unchecked_mut(parent.0 as usize).last = idx;
                Key(*self
                    .keys
                    .get_unchecked(parent.0 as usize)
                    .children_keys
                    .get_unchecked(Idx::new_unchecked(idx + 1)))
            } else {
                let new_id = next().0;

                self.keys
                    .resize(((new_id + 1) as usize).max(self.keys.len()), KeyMapEntry::deleted());

                {
                    let entry = self.keys.get_unchecked_mut(new_id as usize);
                    entry.deleted = false;
                    entry.parent = parent.0;
                    entry.tail = tail;
                }

                let idx = {
                    let parent_entry = self.keys.get_unchecked_mut(parent.0 as usize);
                    let idx = parent_entry.children_keys.add(new_id);
                    let old_len = parent_entry.children_tails.len();
                    parent_entry
                        .children_tails
                        .resize(idx.get().max(old_len), KeyPart::Root.pack());
                    *parent_entry.children_tails.get_unchecked_mut(idx.get() - 1) = tail.pack();
                    idx
                };

                self.keys.get_unchecked_mut(new_id as usize).child_idx = idx;

                Key(new_id)
            }
        }
    }
    pub fn key_parent(&self, key: Key) -> Key { Key(self.keys[key.0 as usize].parent) }
    pub fn key_tail(&self, key: Key) -> KeyPart { self.keys[key.0 as usize].tail }
    pub fn key_debug(&self, key: Key) -> DebuggableKey { DebuggableKey { key, key_map: self } }
    pub fn get_parts(&self, key: Key) -> Vec<KeyPart> {
        let mut parts = Vec::new();
        let mut current = key;
        while current != Key::ROOT {
            parts.push(self.key_tail(current));
            current = self.key_parent(current);
        }
        parts
    }
    pub fn remove(&mut self, key: &Key) {
        // println!("removing {:?}", key);
        self.keys[key.0 as usize].children_keys = Default::default();
        self.keys[key.0 as usize].children_tails = Default::default();
        self.keys[key.0 as usize].deleted = true;

        let parent = self.keys[key.0 as usize].parent;
        if !self.keys[parent as usize].deleted {
            let child_idx = self.keys[key.0 as usize].child_idx;
            // println!("removing myself from parent = {} as idx = {}", parent, child_idx);
            self.keys[parent as usize].children_keys.remove(child_idx);
            self.keys[parent as usize].children_tails[child_idx.get() - 1] = KeyPart::Root.pack();
        } else {
            // println!("parent was already deleted");
        }
        // let sum: usize = self.keys.iter().map(|v|
        // v.children.used_space()).sum(); println!("keys.len() = {},
        // sum = {}", self.keys.len(), sum);
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        let mut keys = vec![KeyMapEntry::deleted(), KeyMapEntry::deleted()];
        keys[Key::ROOT.0 as usize].deleted = false;

        Self { keys }
    }
}
pub struct DebuggableKey<'a> {
    key: Key,
    key_map: &'a KeyMap,
}
impl<'a> Debug for DebuggableKey<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, part) in self.key_map.get_parts(self.key).iter().rev().enumerate() {
            if i != 0 {
                write!(f, ".")?;
            }
            write!(f, "{:?}", part)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyPart {
    Root,

    Fragment { widget_id: u16, location_id: u16 },
    FragmentKey { widget_id: u16, location_id: u16, key: u16 },
}

fn format_location_id(location_id: u16) -> String {
    let column = (location_id & 0b1_1111) * 4;
    let line = location_id >> 5;
    format!("{}:{}", line, column)
}

impl KeyPart {
    pub(crate) fn pack(self) -> i32 {
        unsafe {
            match self {
                // location_id can never be zero, as that would mean we are within 4 chars of the
                // #[widget] thing
                KeyPart::Root => 0,
                KeyPart::Fragment { location_id, .. } => {
                    std::mem::transmute(0b1000_0000_0000_0000u32 | (location_id as u32))
                }
                KeyPart::FragmentKey { location_id, key, .. } => std::mem::transmute(
                    0b0111_1111_1111_1111u32 | ((location_id >> 1) as u32) | ((key as u32) << 15),
                ),
            }
        }
    }
}

impl Debug for KeyPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyPart::Root => write!(f, "Root"),
            KeyPart::Fragment { widget_id, location_id } => {
                let name = internal::name_for_widget(*widget_id);
                write!(f, "{}@{}", name, format_location_id(*location_id))
            }
            KeyPart::FragmentKey { widget_id, location_id, key } => {
                let name = internal::name_for_widget(*widget_id);
                write!(f, "{}<{}>@{}", name, key, format_location_id(*location_id))
            }
        }
    }
}

pub(crate) mod internal {
    use parking_lot::RwLock;

    pub fn name_for_widget(widget_id: u16) -> String {
        WIDGET_INFO.read()[widget_id as usize].name.clone()
    }

    // widget_id,
    // location_id,
    // arg_id
    pub struct WidgetDebugInfo {
        pub name: String,
        pub loc: String,
        // TODO(robin): path, source file, etc
        pub arg_names: Vec<String>,
    }

    lazy_static::lazy_static! {
        pub static ref WIDGET_INFO: RwLock<Vec<WidgetDebugInfo>> = RwLock::new(vec![
            WidgetDebugInfo {
                name: "toplevel".to_string(),
                loc: "".to_string(),
                arg_names: vec![]
            }
        ]);
    }
}
