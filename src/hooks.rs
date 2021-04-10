/* Hooks are part of the core ergonomics of the narui crate: they help to manage all the state of
the GUI application. For implementing them a few nice hacks are employed:
 */

use std::collections::HashMap;
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::ops::Deref;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
struct TreeState(Arc<Mutex<HashMap<String, Box<dyn Any>>>>);

#[derive(Debug)]
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
    fn enter_function(&self, id: &str) -> Context {
        Context {
            path: format!("{}:{}", self.path, id),
            hook_counter: 0,
            tree: self.tree.clone(),
        }
    }

    fn enter_hook<T>(&mut self) -> StateValue<T> {
        self.hook_counter += 1;
        StateValue {
            path: format!("{}->{}", self.path, self.hook_counter),
            tree: self.tree.clone(),
            phantom: PhantomData::default()
        }
    }
}

#[derive(Debug)]
struct StateValue<T>{
    path: String,
    tree: TreeState,
    phantom: PhantomData<T>,
}
impl<T> StateValue<T> where T: 'static + Sync + Send {
    pub fn is_present(&self) -> bool {
        self.tree.0.lock().unwrap().contains_key(&self.path)
    }
    pub fn set(&mut self, new_value: T) {
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

#[macro_export]
macro_rules! use_state {
    ($initial_value: expr) => {
        {
            let mut state_value = context!().enter_hook();
            if !state_value.is_present() {
                state_value.set($initial_value)
            }
            *state_value
        }
    }
}