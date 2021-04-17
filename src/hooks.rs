/* Hooks are part of the core ergonomics of the narui crate: they help to manage all the state of
the GUI application. For implementing them a few nice hacks are employed:
 */

use std::collections::HashMap;
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::ops::Deref;
use std::marker::PhantomData;
use std::fmt::Debug;

#[derive(Clone, Debug)]
struct TreeState(Arc<Mutex<HashMap<String, Box<dyn Any>>>>);

#[derive(Clone, Debug)]
pub struct Context {
    path: String,
    hook_counter: usize,
    tree: TreeState,
}
impl Default for Context {
    fn default() -> Self {
        Context {
            path: String::new(),
            hook_counter: 0,
            tree: TreeState(Arc::new(Mutex::new(HashMap::new()))),
        }
    }
}
impl Context {
    pub fn enter_widget(&self, key: &str) -> Context {
        Context {
            path: format!("{}.{}", self.path, key),
            hook_counter: 0,
            tree: self.tree.clone(),
        }
    }

    pub fn enter_hook<T>(&mut self) -> StateValue<T> {
        self.hook_counter += 1;
        StateValue {
            path: format!("{}->{}", self.path, self.hook_counter),
            tree: self.tree.clone(),
            phantom: PhantomData::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct StateValue<T>{
    path: String,
    tree: TreeState,
    phantom: PhantomData<T>,
}
impl<T> StateValue<T> where T: 'static + Sync + Send {
    pub fn is_present(&self) -> bool {
        self.tree.0.lock().unwrap().contains_key(&self.path)
    }
    pub fn set(&self, new_value: T) {
        self.tree.0.lock().unwrap().insert(self.path.clone(), Box::new(new_value));
    }
}
impl<T> Deref for StateValue<T> where T: 'static + Sync + Send {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // the following unsafe code is inherently unsound but should work for most cases when the
        // hooks api is used correctly.
        unsafe {
            let mut map = self.tree.0.lock().unwrap();
            let boxed = map.remove(&self.path).unwrap();
            let raw = Box::into_raw(boxed);
            map.insert(self.path.clone(), Box::from_raw(raw));
            raw.as_ref().unwrap().downcast_ref().unwrap()
        }
    }
}

// public hooks
pub fn state<T>(initial: T, mut context: Context) -> StateValue<T> where T: 'static + Sync + Send + Debug {
    let state_value: StateValue<T> = context.enter_hook();

    if !state_value.is_present() {
        state_value.set(initial)
    }
    state_value
}
