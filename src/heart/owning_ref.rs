use std::ops::Deref;
use std::marker::PhantomData;

// stolen from owning_ref, added map_assert_stable_address, because shit sucks
pub struct OwningRef<'t, O, T: ?Sized> {
    owner: O,
    reference: *const T,
    marker: PhantomData<&'t T>
}

impl<'t, O, T: ?Sized> OwningRef<'t, O, T> {
    pub unsafe fn new_assert_stable_address(o: O) -> Self
        where O: Deref<Target = T>,
    {
        OwningRef {
            reference: &*o,
            owner: o,
            marker: PhantomData
        }
    }

    pub unsafe fn map_assert_stable_address<F, U: ?Sized>(self, f: F) -> OwningRef<'t, O, U>
        where F: FnOnce(&T) -> &U
    {
        OwningRef {
            reference: f(&self),
            owner: self.owner,
            marker: PhantomData
        }
    }
}


impl<'t, O, T: ?Sized> Deref for OwningRef<'t, O, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            &*self.reference
        }
    }
}
