use std::{
    any::Any,
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

pub type TreeStateInner = HashMap<String, Box<dyn Any>>;
#[derive(Clone, Debug)]
pub struct TreeState(pub Arc<RwLock<TreeStateInner>>);

#[derive(Clone, Debug)]
pub struct Context {
    pub path: String,
    pub tree: TreeState,
}
impl Default for Context {
    fn default() -> Self {
        Context { path: String::new(), tree: TreeState(Arc::new(RwLock::new(HashMap::new()))) }
    }
}
impl Context {
    pub fn enter(&self, key: &str) -> Context {
        Context { path: format!("{}.{}", self.path, key), tree: self.tree.clone() }
    }
}
