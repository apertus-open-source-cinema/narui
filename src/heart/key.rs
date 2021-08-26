use hashbrown::HashMap;
use std::{
    fmt::{Debug, Formatter},
    hash::Hash,
};

type KeyInner = u32;
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Key(KeyInner);

impl Key {
    const ROOT: Key = Key(0);
}

#[derive(Debug)]
pub struct KeyMap {
    id_to_part_parent: HashMap<KeyInner, (KeyPart, KeyInner)>,
    parent_part_to_id: HashMap<(KeyInner, KeyPart), KeyInner>,
}
impl KeyMap {
    pub fn key_with(&mut self, parent: Key, tail: KeyPart) -> Key {
        let query_result = self.parent_part_to_id.get(&(parent.0, tail)).cloned();
        if let Some(id) = query_result {
            Key(id)
        } else {
            let new_id = self.id_to_part_parent.len() as KeyInner;
            self.id_to_part_parent.insert(new_id, (tail, parent.0));
            self.parent_part_to_id.insert((parent.0, tail), new_id);

            Key(new_id)
        }
    }
    pub fn key_parent(&self, key: Key) -> Key {
        Key(self.id_to_part_parent.get(&key.0).unwrap().1)
    }
    pub fn key_tail(&self, key: Key) -> KeyPart { self.id_to_part_parent.get(&key.0).unwrap().0 }
    pub fn key_debug(&self, key: Key) -> DebuggableKey { DebuggableKey { key, key_map: &self } }
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
        if let Some((parent, tail)) = self.id_to_part_parent.remove(&key.0) {
            self.parent_part_to_id.remove(&(tail, parent));
        }
    }
}
impl Default for KeyMap {
    fn default() -> Self {
        let mut id_to_part_parent = HashMap::with_capacity(1024);
        id_to_part_parent.insert(Key::ROOT.0, (KeyPart::Root, Key::ROOT.0));

        let parent_part_to_id = HashMap::with_capacity(1024);

        Self { id_to_part_parent, parent_part_to_id }
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

pub mod internal {
    pub use ctor::ctor;
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
