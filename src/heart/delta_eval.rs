// do partial re evaluation of the changed widget tree

use crate::{
    heart::{Context, Fragment, Key, LayoutObject, LayoutTree},
    FragmentInner,
};
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};
use std::{cell::RefCell, sync::Arc};

// EvaluatedEvalObject is analog to a EvalObject but not lazy and additionally
// contains the dependencies of Node for allowing partial rebuild.
#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct EvaluatedFragment {
    // these fields are part of the original Fragment
    pub key: Key,
    #[derivative(Debug = "ignore")]
    pub gen: Arc<dyn Fn(Context) -> FragmentInner + Send + Sync>,

    // this field is information that is gathered by the delta evaluator
    pub deps: HashSet<Key>,

    // these fields are part of the FragmentResult
    pub layout_object: Option<LayoutObject>,
    pub children: Vec<Arc<RefCell<EvaluatedFragment>>>,
}

// The evaluator outputs nothing but rather communicates with the layouter with
// the LayoutTree trait.
pub struct Evaluator<T: LayoutTree> {
    pub context: Context,
    pub layout_tree: LayoutTreeFilter<T>,
    deps_map: HashMap<Key, Vec<Arc<RefCell<EvaluatedFragment>>>>,
}
impl<T: LayoutTree> Evaluator<T> {
    pub fn new(top_node: Fragment, layout_tree: T) -> Self {
        let layout_tree = LayoutTreeFilter::new(layout_tree);
        let mut evaluator =
            Evaluator { context: Default::default(), layout_tree, deps_map: Default::default() };
        let _root = evaluator.evaluate_unconditional(top_node, evaluator.context.clone());
        evaluator.context.global.write().tree.update_tree();
        evaluator.layout_tree.update();

        evaluator
    }
    fn evaluate_unconditional(
        &mut self,
        fragment: Fragment,
        context: Context,
    ) -> Arc<RefCell<EvaluatedFragment>> {
        let context = context.with_key_widget(fragment.key);
        let evaluated: FragmentInner = (fragment.gen)(context.clone());
        let deps = context.widget_local.used.lock().clone();

        let to_return = Arc::new(RefCell::new(EvaluatedFragment {
            key: fragment.key,
            gen: fragment.gen,
            deps: deps.clone(),
            layout_object: evaluated.layout_object.clone(),
            children: vec![],
        }));

        let children: Vec<_> = evaluated
            .children
            .into_iter()
            .map(|fragment| self.evaluate_unconditional(fragment, context.clone()))
            .collect();
        let children_keys = children.iter().map(|c| c.borrow().key).collect();
        to_return.borrow_mut().children = children;

        self.layout_tree.set_node(fragment.key, evaluated.layout_object, children_keys);

        for key in deps {
            self.deps_map.entry(key).or_default().push(to_return.clone());
        }
        to_return
    }

    pub fn update(&mut self) -> bool {
        for i in 0..32 {
            if !self.update_once() {
                self.layout_tree.update();
                return i != 0;
            }
        }
        panic!("did not converge");
    }
    fn update_once(&mut self) -> bool {
        let mut to_update: HashMap<Key, Arc<RefCell<EvaluatedFragment>>> = HashMap::new();
        let touched_keys = self.context.global.write().tree.update_tree();
        if touched_keys.is_empty() {
            return false;
        }

        let empty_vec = &vec![];
        for key in touched_keys {
            for frag in self.deps_map.get(&key).unwrap_or(empty_vec) {
                to_update.entry(frag.borrow().key).or_insert_with(|| frag.clone());
            }
        }

        for frag in to_update.values() {
            self.re_eval_fragment(frag.clone())
        }

        true
    }
    fn re_eval_fragment(&mut self, frag_cell: Arc<RefCell<EvaluatedFragment>>) {
        let mut frag = frag_cell.borrow_mut();

        let context = self.context.with_key_widget(frag.key);
        let evaluated: FragmentInner = (frag.gen)(context.clone());

        let new_deps = &mut *context.widget_local.used.lock();
        for to_remove in frag.deps.difference(new_deps) {
            self.deps_map.entry(*to_remove).or_default().retain(|x| x.borrow().key != frag.key)
        }
        for to_insert in new_deps.difference(&frag.deps) {
            self.deps_map.entry(*to_insert).or_default().push(frag_cell.clone())
        }
        frag.deps = std::mem::replace(new_deps, HashSet::new());

        frag.layout_object = evaluated.layout_object;

        let old_children = frag.children.clone();
        frag.children = evaluated
            .children
            .iter()
            .map(|f| {
                frag.children
                    .iter()
                    .find(|candidate| candidate.borrow().key == f.key)
                    .cloned()
                    .unwrap_or_else(|| self.evaluate_unconditional(f.clone(), context.clone()))
            })
            .collect();

        let children_keys = frag.children.iter().map(|child| child.borrow().key).collect();
        self.layout_tree.set_node(frag.key, frag.layout_object.clone(), children_keys);

        for child in old_children {
            if !frag.children.iter().any(|candidate| candidate.borrow().key == child.borrow().key) {
                self.remove_tree(child.as_ref(), false)
            }
        }
    }
    fn remove_tree(&mut self, tree: &RefCell<EvaluatedFragment>, top: bool) {
        // TODO handle removal of layout objects
        let frag = tree.borrow();
        if top {
            self.context.global.write().tree.remove(frag.key);
        }
        for child in frag.children.iter() {
            self.remove_tree(child, false);
        }
        for to_remove in frag.deps.iter() {
            self.deps_map.entry(*to_remove).or_default().retain(|x| x.borrow().key != frag.key)
        }
    }
}


pub struct LayoutTreeFilter<T: LayoutTree> {
    pub layout_tree: T,
    children_map: HashMap<Key, Vec<Key>>,
    parent_map: HashMap<Key, Key>,
    is_layout_object: HashMap<Key, bool>,
    dirty_keys: HashSet<Key>,
}
impl<T: LayoutTree> LayoutTreeFilter<T> {
    pub fn new(layout_tree: T) -> Self {
        Self {
            layout_tree,
            children_map: Default::default(),
            parent_map: Default::default(),
            dirty_keys: Default::default(),
            is_layout_object: Default::default(),
        }
    }

    pub fn set_node(
        &mut self,
        key: Key,
        layout_object: Option<LayoutObject>,
        children_keys: Vec<Key>,
    ) {
        for child_key in children_keys.clone() {
            assert!(child_key != key, "{:?}", key);
            self.parent_map.insert(child_key, key);
        }
        self.children_map.insert(key, children_keys);
        self.dirty_keys.insert(key);

        self.is_layout_object.insert(key, layout_object.is_some());
        if let Some(layout_object) = layout_object {
            self.layout_tree.set_node(key, layout_object);
        }
    }
    pub fn remove_node(&mut self, key: Key) {
        self.layout_tree.remove_node(key);
        self.is_layout_object.remove(&key);
        self.children_map.remove(&key);
        self.parent_map.remove(&key);
    }
    pub fn update(&mut self) {
        let mut to_update_parent_nodes = HashSet::new();
        let dirty_keys: Vec<_> = self.dirty_keys.drain().collect();
        for dirty_key in dirty_keys {
            to_update_parent_nodes.insert(self.get_layout_parent(&dirty_key));
        }
        for parent in to_update_parent_nodes.drain() {
            self.layout_tree.set_children(parent, self.get_layout_children(parent).into_iter());
        }
    }

    fn get_layout_parent(&self, key: &Key) -> Key {
        if let Some(parent) = self.parent_map.get(key) {
            if self.is_layout_object[parent] {
                *parent
            } else {
                self.get_layout_parent(parent)
            }
        } else {
            Key::default()
        }
    }
    fn get_layout_children(&self, parent: Key) -> Vec<Key> {
        let mut to_return = Vec::with_capacity(10);
        for k in self.children_map[&parent].iter() {
            if self.is_layout_object[k] {
                to_return.push(*k)
            } else {
                to_return.append(&mut self.get_layout_children(*k))
            }
        }
        to_return
    }
}
