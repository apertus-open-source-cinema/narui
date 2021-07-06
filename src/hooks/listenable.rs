use crate::heart::{Context, Key, PatchedTree};
use parking_lot::RwLockReadGuard;
use std::{marker::PhantomData, ops::Deref};


pub trait ContextListenable {
    fn listenable_key<T: Send + Sync + 'static>(&self, key: Key, initial: T) -> Listenable<T>;
    fn listenable<T: Send + Sync + 'static>(&self, initial: T) -> Listenable<T>;

    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T);
    fn shout_unconditional<T: Send + Sync + 'static>(&self, listenable: Listenable<T>, initial: T);

    fn listen<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone;
    fn listen_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T>;
}
impl ContextListenable for Context {
    fn listenable_key<T: Send + Sync + 'static>(&self, key: Key, initial: T) -> Listenable<T> {
        let listenable = Listenable { key, phantom_data: Default::default() };
        if self.global.read().get(listenable.key).is_none() {
            self.shout_unconditional(listenable, initial)
        };
        listenable
    }
    fn listenable<T: Send + Sync + 'static>(&self, initial: T) -> Listenable<T> {
        self.listenable_key(self.key_for_hook(), initial)
    }

    fn shout<T: Send + Sync + 'static + PartialEq>(&self, listenable: Listenable<T>, new_value: T) {
        let mut lock = self.global.write();
        match lock.get(listenable.key) {
            None => lock.set(listenable.key, Box::new(new_value)),
            Some(old_value) => {
                if (&**old_value).downcast_ref::<T>().unwrap() != &new_value {
                    lock.set(listenable.key, Box::new(new_value))
                }
            }
        }
    }
    fn shout_unconditional<T: Send + Sync + 'static>(&self, listenable: Listenable<T>, new_value: T) {
        let mut lock = self.global.write();
        lock.set(listenable.key, Box::new(new_value))
    }

    fn listen<T: Send + Sync + 'static>(&self, listenable: Listenable<T>) -> T
    where
        T: Clone,
    {
        self.widget_local.mark_used(listenable.key);
        self.global.read().get(listenable.key).unwrap().downcast_ref::<T>().unwrap().clone()
    }

    fn listen_ref<T: Send + Sync>(&self, listenable: Listenable<T>) -> ListenableGuard<T> {
        ListenableGuard {
            rw_lock_guard: self.global.read(),
            path: listenable.key,
            phantom: Default::default(),
        }
    }
}

#[macro_export]
macro_rules! shout_ {
    ($context:ident, $listenable:ident, $value:expr) => {{
        fn constrain_type<T>(_a: &Listenable<T>, _b: &T) {}
        constrain_type(&$listenable, &$value);
        let mut lock = $context.global.write();
        match lock.get($listenable.key) {
            None => lock.set($listenable.key, Box::new($value)),
            Some(old_value) => {
                let old = (&**old_value).downcast_ref().unwrap();
                fn constrain_type<T>(_a: &T, _b: &T) {}
                constrain_type(old, &$value);
                if !all_eq!(old, &$value) {
                    lock.set($listenable.key, Box::new($value));
                }
            }
        }
    }};
}
pub use shout_ as shout;

pub struct Listenable<T> {
    pub key: Key,
    phantom_data: PhantomData<T>,
}
impl<T> Listenable<T> {
    pub unsafe fn uninitialized(key: Key) -> Self {
        Listenable { key, phantom_data: Default::default() }
    }
}
impl<T> Clone for Listenable<T> {
    fn clone(&self) -> Self { Self { key: self.key, phantom_data: Default::default() } }
}
impl<T> Copy for Listenable<T> {}

pub struct ListenableGuard<'l, T> {
    rw_lock_guard: RwLockReadGuard<'l, PatchedTree>,
    phantom: PhantomData<T>,
    path: Key,
}
impl<'l, T: 'static> Deref for ListenableGuard<'l, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.rw_lock_guard.get(self.path).unwrap().downcast_ref().unwrap()
    }
}
