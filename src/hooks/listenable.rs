use crate::heart::{Key, Context, TreeStateInner, PatchedTree};
use std::marker::PhantomData;
use parking_lot::RwLockReadGuard;
use std::ops::Deref;

pub trait ContextListenable {
    fn listenable_key<T>(&self, key: Key, initial: T) -> Listenable<T>;
    fn listenable<T>(&self, initial: T) -> Listenable<T>;

    fn shout<T>(&self, listenable: &Listenable<T>, new_value: T);

    fn listen<T>(&self, listenable: &Listenable<T>) -> T where T: Clone;
    fn listen_ref<T>(&self, listenable: &Listenable<T>) -> ListenableGuard<T>;
    fn listen_changed<T>(&self, listenable: &Listenable<T>) -> bool;
}
impl ContextListenable for Context {
    fn listenable_key<T>(&self, key: Key, initial: T) -> Listenable<T> {
        Listenable {
            key, phantom_data: Default::default()
        }
    }

    fn listenable<T>(&self, initial: T) -> Listenable<T> {
        Listenable {
            key: self.key_for_hook(),
            phantom_data: Default::default()
        }
    }

    fn shout<T>(&self, listenable: &Listenable<T>, new_value: T) {
        self.global.write().set(listenable.key.clone(), Box::new(new_value));
    }

    fn listen<T>(&self, listenable: &Listenable<T>) -> T where T: Clone {
        if !self.is_present(&listenable.key) {
            panic!("no entry found for key {:?}", &listenable.key);
        }
        self.widget_local.mark_used(listenable.key);
        self.global.tree.read()[&listenable.key].downcast_ref::<T>().unwrap().clone()
    }

    fn listen_ref<T>(&self, listenable: &Listenable<T>) -> ListenableGuard<T> {
        ListenableGuard {
            rw_lock_guard: self.global.read(),
            path: listenable.key,
            phantom: Default::default(),
        }
    }

    fn listen_changed<T>(&self, listenable: &Listenable<T>) -> bool {
        return self.global.read().is_updated(listenable.key, |a, b| {
            let a: &T = a.downcast_ref().unwrap();
            let b: &T = b.downcast_ref().unwrap();
            a == b
        })
    }
}

pub struct Listenable<T> {
    key: Key,
    phantom_data: PhantomData<T>,
}
impl<T> Clone for Listenable<T> {
    fn clone(&self) -> Self {
        Self { key: self.key, phantom_data: Default::default() }
    }
}
impl<T> Copy for Listenable<T> {}

pub struct ListenableGuard<'l, T> {
    rw_lock_guard: RwLockReadGuard<'l, PatchedTree>,
    phantom: PhantomData<T>,
    path: Key,
}
impl<'l, T: 'static> Deref for ListenableGuard<'l, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { self.rw_lock_guard[&self.path].downcast_ref().unwrap() }
}
