use std::{
    mem::{ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{Deref, Index, IndexMut},
};

pub type Idx = NonZeroUsize;

#[derive(Debug, Clone)]
struct Slot<T> {
    data: ManuallyDrop<T>,
    next_free: Option<Idx>,
}

#[derive(Debug, Clone)]
pub struct FreeList<T> {
    entries: Vec<Slot<T>>,
    next_free: Option<Idx>,
    used_slot_two: bool,
}

impl<T> Default for FreeList<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Drop for FreeList<T> {
    fn drop(&mut self) {
        let raw = self.entries.as_mut_ptr();
        let start = if self.used_slot_two { 1 } else { 2 };
        unsafe {
            std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                raw.add(start),
                self.entries.len() - start,
            ));
            self.entries.set_len(0);
        }
    }
}

impl<T> FreeList<T> {
    pub fn new() -> Self {
        let mut data = vec![];

        // push a dummy thing to idx zero so we can use NonZeroUsize
        #[allow(clippy::uninit_assumed_init)]
        data.push(Slot { data: unsafe { MaybeUninit::uninit().assume_init() }, next_free: None });

        // push a empty slot to use as first entry
        #[allow(clippy::uninit_assumed_init)]
        data.push(Slot { data: unsafe { MaybeUninit::uninit().assume_init() }, next_free: None });

        let next_free = Some(unsafe { Idx::new_unchecked(1) });

        Self { entries: data, next_free, used_slot_two: false }
    }

    pub fn add(&mut self, data: T) -> Idx {
        match self.next_free {
            None => {
                let idx = self.entries.len();
                self.entries.push(Slot { data: ManuallyDrop::new(data), next_free: None });
                unsafe { Idx::new_unchecked(idx) }
            }
            Some(idx) => {
                self.next_free = self.entries[idx.get()].next_free.take();
                let mut old =
                    std::mem::replace(&mut self.entries[idx.get()].data, ManuallyDrop::new(data));
                if (idx.get() == 1) && !self.used_slot_two {
                    self.used_slot_two = true;
                } else {
                    unsafe { ManuallyDrop::drop(&mut old) }
                }
                idx
            }
        }
    }

    pub fn remove(&mut self, idx: Idx) {
        self.entries[idx.get()].next_free = self.next_free;
        self.next_free = Some(idx);
    }

    pub fn remove_replace(&mut self, idx: Idx, sentinel: T) {
        self.entries[idx.get()].next_free = self.next_free;
        unsafe {
            ManuallyDrop::drop(&mut std::mem::replace(
                &mut self.entries[idx.get()].data,
                ManuallyDrop::new(sentinel),
            ));
        }
        self.next_free = Some(idx);
    }

    // potentially accesses data already removed
    pub unsafe fn find(&self, predicate: impl FnMut(&&T) -> bool) -> Option<&T> {
        self.entries
            .iter()
            .skip(if self.used_slot_two { 1 } else { 2 })
            .map(|v| &*v.data)
            .find(predicate)
    }

    // this only works until you push new items
    pub unsafe fn removed(&mut self, idx: Idx) -> bool {
        self.entries[idx.get()].next_free.is_some() || self.next_free == Some(idx)
    }

    // make sure to only touch elements one and above
    pub unsafe fn iter_raw(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|v| v.data.deref())
    }

    pub fn used_space(&self) -> usize { self.entries.len() }

    pub unsafe fn get_unchecked(&self, idx: Idx) -> &T {
        &self.entries.get_unchecked(idx.get()).data
    }

    pub unsafe fn get_unchecked_mut(&mut self, idx: Idx) -> &mut T {
        &mut self.entries.get_unchecked_mut(idx.get()).data
    }
}

impl<T> Index<Idx> for FreeList<T> {
    type Output = T;

    fn index(&self, index: Idx) -> &Self::Output { &*self.entries[index.get()].data }
}

impl<T> IndexMut<Idx> for FreeList<T> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        &mut *self.entries[index.get()].data
    }
}
