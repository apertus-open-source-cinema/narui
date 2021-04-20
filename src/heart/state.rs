use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
struct TreeState(Arc<Mutex<HashMap<String, Box<dyn Any>>>>);

#[derive(Clone, Debug)]
pub struct Context {
    path: String,
    tree: TreeState,
}
impl Default for Context {
    fn default() -> Self {
        Context { path: String::new(), tree: TreeState(Arc::new(Mutex::new(HashMap::new()))) }
    }
}
impl Context {
    pub fn enter_widget(&self, key: &str) -> Context {
        Context { path: format!("{}.{}", self.path, key), tree: self.tree.clone() }
    }

    pub fn enter_hook<T>(&self, key: &str) -> StateValue<T> {
        StateValue {
            path: format!("{}->{}", self.path, key),
            tree: self.tree.clone(),
            phantom: PhantomData::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StateValue<T> {
    path: String,
    tree: TreeState,
    phantom: PhantomData<T>,
}
impl<T> StateValue<T>
where
    T: 'static + Sync + Send,
{
    pub fn is_present(&self) -> bool { self.tree.0.lock().unwrap().contains_key(&self.path) }
    pub fn set(&self, new_value: T) {
        self.tree.0.lock().unwrap().insert(self.path.clone(), Box::new(new_value));
    }
}
impl<T> Deref for StateValue<T>
where
    T: 'static + Sync + Send,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // the following unsafe code is inherently unsound but should work for most
        // cases when the hooks api is used correctly.
        unsafe {
            let mut map = self.tree.0.lock().unwrap();
            let boxed = map.remove(&self.path).unwrap();
            let raw = Box::into_raw(boxed);
            map.insert(self.path.clone(), Box::from_raw(raw));
            raw.as_ref().unwrap().downcast_ref().unwrap()
        }
    }
}
