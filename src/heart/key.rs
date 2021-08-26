use std::{
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
};


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

#[cfg(debug_assertions)]
const KEY_BYTES: usize = 128;

#[cfg(not(debug_assertions))]
const KEY_BYTES: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    data: [u8; KEY_BYTES],
    // points to the last byte of the current KeyPart
    pos: usize,
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

#[derive(Clone, Copy, PartialEq)]
pub enum KeyPart {
    Uninitialized,
    Root,

    Hook(u16),

    Fragment { widget_id: u16, location_id: u16 },
    FragmentKey { widget_id: u16, location_id: u16, key: u16 },
}

const UNINITIALIZED: u8 = 0;
const HOOK: u8 = 1;
const FRAGMENT: u8 = 2;
const FRAGMENT_KEY: u8 = 3;

impl KeyPart {
    pub fn push_to(self, key: &mut Key) {
        assert!(key.pos + 8 < KEY_BYTES);
        assert_ne!(self, KeyPart::Root);
        match self {
            KeyPart::Uninitialized => {
                key.pos += 1;
                key.data[key.pos] = UNINITIALIZED;
            }
            KeyPart::Hook(idx) => {
                key.pos += 2;
                key.data[key.pos - 1] = ((idx >> 6) & 0xff) as u8;
                key.data[key.pos] = (((idx << 2) & 0xff) as u8 | HOOK) & 0xff;
            }
            #[cfg(debug_assertions)]
            KeyPart::Fragment { widget_id, location_id } => {
                key.pos += 4;
                key.data[key.pos - 3] = (location_id & 0xff) as u8;
                key.data[key.pos - 2] = ((location_id >> 8) & 0xff) as u8;
                key.data[key.pos - 1] = ((widget_id >> 6) & 0xff) as u8;
                key.data[key.pos] = (((widget_id << 2) & 0xff) as u8 | FRAGMENT) & 0xff;
            }
            #[cfg(not(debug_assertions))]
            KeyPart::Fragment { widget_id, location_id } => {
                key.pos += 2;
                key.data[key.pos - 1] = ((location_id >> 6) & 0xff) as u8;
                key.data[key.pos] = (((location_id << 2) & 0xff) as u8 | FRAGMENT) & 0xff;
            }
            #[cfg(debug_assertions)]
            KeyPart::FragmentKey { widget_id, location_id, key: k } => {
                key.pos += 6;
                key.data[key.pos - 5] = ((k >> 0) & 0xff) as u8;
                key.data[key.pos - 4] = ((k >> 8) & 0xff) as u8;
                key.data[key.pos - 3] = (location_id & 0xff) as u8;
                key.data[key.pos - 2] = ((location_id >> 8) & 0xff) as u8;
                key.data[key.pos - 1] = ((widget_id >> 6) & 0xff) as u8;
                key.data[key.pos] = (((widget_id << 2) & 0xff) as u8 | FRAGMENT_KEY) & 0xff;
            }
            #[cfg(not(debug_assertions))]
            KeyPart::FragmentKey { widget_id, location_id, key: k } => {
                key.pos += 4;
                key.data[key.pos - 3] = ((k >> 0) & 0xff) as u8;
                key.data[key.pos - 2] = ((k >> 8) & 0xff) as u8;
                key.data[key.pos - 1] = ((location_id >> 6) & 0xff) as u8;
                key.data[key.pos] = (((location_id << 2) & 0xff) as u8 | FRAGMENT_KEY) & 0xff;
            }
            KeyPart::Root => { unreachable!() }
        }
        // dbg!(self, &key.pos, &key);
    }

    pub fn decode_all_reversed(key: &Key) -> Vec<KeyPart> {
        let mut pos = key.pos;
        let mut parts = vec![];
        while pos > 0 {
            match key.data[pos] & 0b11 {
                UNINITIALIZED => {
                    parts.push(KeyPart::Uninitialized);
                    pos -= 1;
                }
                HOOK => {
                    let mut idx = 0u16;
                    idx |= (key.data[pos - 1] as u16) << 6;
                    idx |= key.data[pos] as u16 >> 2;
                    parts.push(KeyPart::Hook(idx));
                    pos -= 2;
                }
                #[cfg(debug_assertions)]
                FRAGMENT => {
                    let mut widget_id = 0u16;
                    let mut location_id = 0u16;

                    location_id |= key.data[pos - 3] as u16;
                    location_id |= (key.data[pos - 2] as u16) << 8;
                    widget_id |= (key.data[pos - 1] as u16) << 6;
                    widget_id |= key.data[pos] as u16 >> 2;

                    parts.push(KeyPart::Fragment { widget_id, location_id });
                    pos -= 4;
                }
                #[cfg(not(debug_assertions))]
                FRAGMENT => {
                    let mut widget_id = 0u16;
                    let mut location_id = 0u16;

                    location_id |= (key.data[pos - 1] as u16) << 6;
                    location_id |= key.data[pos] as u16 >> 2;

                    parts.push(KeyPart::Fragment { widget_id, location_id });
                    pos -= 2;
                }
                #[cfg(debug_assertions)]
                FRAGMENT_KEY => {
                    let mut widget_id = 0u16;
                    let mut location_id = 0u16;
                    let mut k = 0u16;
                    k |= (key.data[pos - 5] as u16) << 0;
                    k |= (key.data[pos - 4] as u16) << 8;
                    location_id |= key.data[pos - 3] as u16;
                    location_id |= (key.data[pos - 2] as u16) << 8;
                    widget_id |= (key.data[pos - 1] as u16) << 6;
                    widget_id |= key.data[pos] as u16 >> 2;

                    parts.push(KeyPart::FragmentKey { key: k, widget_id, location_id });
                    pos -= 6;
                }
                #[cfg(not(debug_assertions))]
                FRAGMENT_KEY => {
                    let mut widget_id = 0u16;
                    let mut location_id = 0u16;
                    let mut k = 0u16;
                    k |= (key.data[pos - 5] as u16) << 0;
                    k |= (key.data[pos - 4] as u16) << 8;
                    location_id |= (key.data[pos - 1] as u16) << 6;
                    location_id |= key.data[pos] as u16 >> 2;

                    parts.push(KeyPart::FragmentKey { key: k, widget_id, location_id });
                    pos -= 4;
                }
                unk => panic!("unknown key tag {}", unk),
            }
        }

        assert_eq!(key.data[0] & 0b11, UNINITIALIZED);
        parts.push(KeyPart::Root);

        parts
    }

    fn last_part_size(key: &Key) -> usize {
        match key.data[key.pos] & 0b11 {
            UNINITIALIZED => 1,
            HOOK => 2,
            #[cfg(debug_assertions)]
            FRAGMENT => 4,
            #[cfg(not(debug_assertions))]
            FRAGMENT => 2,
            #[cfg(debug_assertions)]
            FRAGMENT_KEY => 6,
            #[cfg(not(debug_assertions))]
            FRAGMENT_KEY => 4,
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
                let name = internal::name_for_widget(*widget_id);
                write!(f, "Fragment_{}_{}", name, location_id)
            }
            KeyPart::FragmentKey { widget_id, location_id, key } => {
                let name = internal::name_for_widget(*widget_id);
                write!(f, "FragmentKey_{}_{}_{}", name, location_id, key)
            }
        }
    }
}
