// do partial re evaluation of the changed widget tree

use crate::heart::{Context, EvalObject, Key, LayoutObject, LayoutTree};
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};
use std::{cell::RefCell, sync::Arc};


type Deps = HashSet<Key>;
// EvaluatedEvalObject is analog to a EvalObject but not lazy and additionally
// contains the dependencies of Node for allowing partial rebuild.
#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct EvaluatedEvalObject {
    pub key: Key,
    pub deps: Deps,
    #[derivative(Debug = "ignore")]
    pub gen: Arc<dyn Fn(Context) -> EvalObject + Send + Sync>,
    pub layout_object: Option<LayoutObject>,
    pub children: Vec<Arc<RefCell<EvaluatedEvalObject>>>,
}

// The evaluator outputs nothing but rather communicates with the layouter with
// the LayoutTree trait.
pub struct Evaluator<T: LayoutTree> {
    pub context: Context,
    pub layout_tree: LayoutTreeFilter<T>,
    deps_map: HashMap<Key, Vec<Arc<RefCell<EvaluatedEvalObject>>>>,
}
impl<T: LayoutTree> Evaluator<T> {
    pub fn new(top_node: EvalObject, layout_tree: T) -> Self {
        let layout_tree = LayoutTreeFilter::new(layout_tree);
        let mut evaluator =
            Evaluator { context: Default::default(), layout_tree, deps_map: Default::default() };
        let top_gen = Arc::new(move |_context| top_node.clone());
        let _root = evaluator.evaluate_unconditional(top_gen, evaluator.context.clone());
        evaluator.context.global.write().update_tree();
        evaluator.layout_tree.update();

        evaluator
    }
    fn evaluate_unconditional(
        &mut self,
        gen: Arc<dyn Fn(Context) -> EvalObject + Send + Sync>,
        context: Context,
    ) -> Arc<RefCell<EvaluatedEvalObject>> {
        let key = context.widget_local.key;
        let evaluated: EvalObject = gen(context.clone());
        let deps = context.widget_local.used.lock().clone();

        let to_return = Arc::new(RefCell::new(EvaluatedEvalObject {
            key,
            gen,
            deps: deps.clone(),
            layout_object: evaluated.layout_object.clone(),
            children: vec![],
        }));

        let mut children_keys = Vec::with_capacity(10);
        let children: Vec<_> = evaluated
            .children
            .into_iter()
            .map(|(key_part, gen)| {
                let key = context.widget_local.key.with(key_part);
                children_keys.push(key);
                let context = context.with_key_widget(key);
                self.evaluate_unconditional(gen, context)
            })
            .collect();
        to_return.borrow_mut().children = children;

        self.layout_tree.set_node(key, evaluated.layout_object, children_keys);

        for key in deps {
            self.deps_map.entry(key).or_default().push(to_return.clone());
        }
        to_return
    }

    pub fn update(&mut self) -> bool {
        for i in 0..32 {
            if !self.update_once() {
                self.layout_tree.update();
                return if i == 0 { false } else { true };
            }
        }
        panic!("did not converge");
    }
    fn update_once(&mut self) -> bool {
        let mut to_update: HashMap<Key, Arc<RefCell<EvaluatedEvalObject>>> = HashMap::new();
        let touched_keys = self.context.global.write().update_tree();
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
    fn re_eval_fragment(&mut self, frag_cell: Arc<RefCell<EvaluatedEvalObject>>) {
        let mut frag = frag_cell.borrow_mut();

        let context = self.context.with_key_widget(frag.key);
        let evaluated: EvalObject = (frag.gen)(context.clone());

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
            .map(|(key_part, child)| {
                frag.children
                    .iter()
                    .find(|candidate| candidate.borrow().key.last_part() == *key_part)
                    .cloned()
                    .unwrap_or_else(|| {
                        self.evaluate_unconditional(
                            child.clone(),
                            context.with_key_widget(context.widget_local.key.with(*key_part)),
                        )
                    })
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
    fn remove_tree(&mut self, tree: &RefCell<EvaluatedEvalObject>, top: bool) {
        // TODO handle removal of layout objects
        let frag = tree.borrow();
        if top {
            self.context.global.write().remove(frag.key);
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
                return *parent;
            } else {
                return self.get_layout_parent(parent);
            }
        } else {
            return Key::default();
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
