use std::{
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
};


pub mod internal {
    pub use ctor::ctor;
    use parking_lot::RwLock;

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

const KEY_BYTES: usize = 128;

#[derive(Clone, Copy)]
pub struct Key {
    data: [u8; KEY_BYTES],
    // points to the last byte of the current KeyPart
    pos: usize,
}

impl Eq for Key {}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        (self.pos == other.pos) && (self.data[..self.pos + 1] == other.data[..other.pos + 1])
    }

    fn ne(&self, other: &Self) -> bool {
        (self.pos != other.pos) || (self.data[..self.pos + 1] != other.data[..other.pos + 1])
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) { self.data[..self.pos + 1].hash(state); }
}

impl Key {
    pub fn with(&self, tail: KeyPart) -> Self {
        let mut new = self.clone();
        tail.push_to(&mut new);
        new
    }

    pub fn parent(&self) -> Self {
        // println!("calling parent");
        // dbg!(self, &self.pos, &self);
        let mut new = Key::default();
        let last_size = KeyPart::last_part_size(&self);
        new.pos = self.pos - last_size;
        new.data[..new.pos + 1].clone_from_slice(&self.data[..new.pos + 1]);
        // dbg!(self, &self.pos, &self);
        new
    }

    pub fn starts_with(&self, start: &Key) -> bool {
        for i in 0..start.pos {
            if self.data[i] != start.data[i] {
                return false;
            }
        }
        true
    }
}

impl Default for Key {
    fn default() -> Self {
        let mut data = [0; KEY_BYTES];
        data[0] = ROOT;
        Self { data, pos: 0 }
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for part in KeyPart::decode_all_reversed(self).iter().rev() {
            if !first {
                write!(f, ".")?;
            }
            write!(f, "{:?}", part)?;
            first = false;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum KeyPart {
    Uninitialized,
    Root,

    Hook(u16),

    Fragment { widget_id: u16, location_id: u16 },
    FragmentKey { widget_id: u16, location_id: u16, key: u32 },
}

const UNINITIALIZED: u8 = 0;
const ROOT: u8 = 1;
const HOOK: u8 = 2;
const FRAGMENT: u8 = 3;
const FRAGMENT_KEY: u8 = 4;

impl KeyPart {
    pub fn push_to(self, key: &mut Key) {
        // dbg!(self, &key.pos, &key);
        match self {
            KeyPart::Uninitialized => {
                key.pos += 1;
                key.data[key.pos] = UNINITIALIZED;
            }
            KeyPart::Root => {
                key.pos += 1;
                key.data[key.pos] = ROOT;
            }
            KeyPart::Hook(idx) => {
                key.pos += 2;
                key.data[key.pos - 1] = ((idx >> 4) & 0xff) as u8;
                key.data[key.pos] = (((idx << 4) & 0xff) as u8 | HOOK) & 0xff;
            }
            KeyPart::Fragment { widget_id, location_id } => {
                key.pos += 4;
                key.data[key.pos - 3] = (location_id & 0xff) as u8;
                key.data[key.pos - 2] = ((location_id >> 8) & 0xff) as u8;
                key.data[key.pos - 1] = ((widget_id >> 4) & 0xff) as u8;
                key.data[key.pos] = (((widget_id << 4) & 0xff) as u8 | FRAGMENT) & 0xff;
            }
            KeyPart::FragmentKey { widget_id, location_id, key: k } => {
                key.pos += 8;
                key.data[key.pos - 7] = ((k >> 0) & 0xff) as u8;
                key.data[key.pos - 6] = ((k >> 8) & 0xff) as u8;
                key.data[key.pos - 5] = ((k >> 16) & 0xff) as u8;
                key.data[key.pos - 4] = ((k >> 24) & 0xff) as u8;
                key.data[key.pos - 3] = (location_id & 0xff) as u8;
                key.data[key.pos - 2] = ((location_id >> 8) & 0xff) as u8;
                key.data[key.pos - 1] = ((widget_id >> 4) & 0xff) as u8;
                key.data[key.pos] = (((widget_id << 4) & 0xff) as u8 | FRAGMENT_KEY) & 0xff;
            }
        }
        // dbg!(self, &key.pos, &key);
    }

    pub fn decode_all_reversed(key: &Key) -> Vec<KeyPart> {
        let mut pos = key.pos;
        let mut parts = vec![];
        while pos > 0 {
            match key.data[pos] & 0xf {
                ROOT => {
                    parts.push(KeyPart::Root);
                    pos -= 1;
                }
                UNINITIALIZED => {
                    parts.push(KeyPart::Uninitialized);
                    pos -= 1;
                }
                HOOK => {
                    let mut idx = 0u16;
                    idx |= (key.data[pos - 1] as u16) << 4;
                    idx |= key.data[pos] as u16 >> 4;
                    parts.push(KeyPart::Hook(idx));
                    pos -= 2;
                }
                FRAGMENT => {
                    let mut widget_id = 0u16;
                    let mut location_id = 0u16;

                    location_id |= key.data[pos - 3] as u16;
                    location_id |= (key.data[pos - 2] as u16) << 8;
                    widget_id |= (key.data[pos - 1] as u16) << 4;
                    widget_id |= key.data[pos] as u16 >> 4;

                    parts.push(KeyPart::Fragment { widget_id, location_id });
                    pos -= 4;
                }
                FRAGMENT_KEY => {
                    let mut widget_id = 0u16;
                    let mut location_id = 0u16;
                    let mut k = 0u32;
                    k |= (key.data[pos - 7] as u32) << 0;
                    k |= (key.data[pos - 6] as u32) << 8;
                    k |= (key.data[pos - 5] as u32) << 16;
                    k |= (key.data[pos - 4] as u32) << 24;
                    location_id |= key.data[pos - 3] as u16;
                    location_id |= (key.data[pos - 2] as u16) << 8;
                    widget_id |= (key.data[pos - 1] as u16) << 4;
                    widget_id |= key.data[pos] as u16 >> 4;

                    parts.push(KeyPart::FragmentKey { key: k, widget_id, location_id });
                    pos -= 8;
                }
                unk => panic!("unknown key tag {}", unk),
            }
        }

        assert_eq!(key.data[0] & 0xf, ROOT);
        parts.push(KeyPart::Root);

        parts
    }

    fn last_part_size(key: &Key) -> usize {
        match key.data[key.pos] & 0xf {
            ROOT => 1,
            UNINITIALIZED => 1,
            HOOK => 2,
            FRAGMENT => 4,
            FRAGMENT_KEY => 8,
            unk => panic!("unknown key tag {}", unk),
        }
    }
}

impl Default for KeyPart {
    fn default() -> Self { KeyPart::Uninitialized }
}

impl Debug for KeyPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyPart::Uninitialized => write!(f, "Uninitialized"),
            KeyPart::Root => write!(f, "Root"),
            KeyPart::Hook(number) => write!(f, "Hook_{}", number),
            KeyPart::Fragment { widget_id, location_id } => {
                write!(f, "Fragment_{}_{}", widget_id, location_id)
            }
            KeyPart::FragmentKey { widget_id, location_id, key } => {
                write!(f, "FragmentKey_{}_{}_{}", widget_id, location_id, key)
            }
        }
    }
}
