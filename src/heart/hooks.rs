/* Hooks are part of the heart ergonomics of the narui crate: they help to manage all the state of
the GUI application. For implementing them a few nice hacks are employed:
 */

use super::state::*;
use crate::heart::TreeStateInner;
use std::{fmt::Debug, marker::PhantomData, ops::Deref, sync::RwLockReadGuard};


#[derive(Clone, Debug)]
pub struct StateValue<T> {
    context: Context,
    phantom: PhantomData<T>,
}
impl<T> StateValue<T> {
    pub fn new(context: Context, key: &str) -> Self {
        StateValue { context: context.enter(key), phantom: PhantomData::default() }
    }
}
impl<T> StateValue<T>
where
    T: 'static + Sync + Send,
{
    pub fn is_present(&self) -> bool {
        self.context.tree.0.read().unwrap().contains_key(&self.context.key)
    }
    pub fn set(&self, new_value: T) {
        self.context.tree.0.write().unwrap().insert(self.context.key.clone(), Box::new(new_value));
    }
    pub fn get_ref(&self) -> StateValueGuard<T> {
        StateValueGuard {
            rw_lock_guard: self.context.tree.0.read().unwrap(),
            path: self.context.key.clone(),
            phantom: Default::default(),
        }
    }
}
impl<T> StateValue<T>
where
    T: Clone + 'static + Sync + Send,
{
    pub fn get(&self) -> T {
        self.context.tree.0.read().unwrap()[&self.context.key].downcast_ref::<T>().unwrap().clone()
    }
}

pub struct StateValueGuard<'l, T> {
    rw_lock_guard: RwLockReadGuard<'l, TreeStateInner>,
    phantom: PhantomData<T>,
    path: String,
}
impl<'l, T> Deref for StateValueGuard<'l, T>
where
    T: 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target { self.rw_lock_guard[&self.path].downcast_ref().unwrap() }
}
pub fn state<T>(initial: T, context: Context) -> StateValue<T>
where
    T: 'static + Sync + Send + Debug,
{
    let state_value: StateValue<T> = StateValue::new(context, "state");
    if !state_value.is_present() {
        state_value.set(initial)
    }
    state_value
}


pub fn rise_detector(to_probe: StateValue<bool>, callback: impl Fn() -> (), context: Context) {
    let last = state(false, context);
    if to_probe.get() && !last.get() {
        callback();
    }
    last.set(to_probe.get());
}
