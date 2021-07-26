// do partial re evaluation of the changed widget tree

use crate::heart::{Context, EvalObject, Key, LayoutObject, LayoutTree};
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};
use parking_lot::Mutex;
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
    layout_tree: Arc<Mutex<T>>,
    deps_map: HashMap<Key, Vec<Arc<RefCell<EvaluatedEvalObject>>>>,
}
impl<T: LayoutTree> Evaluator<T> {
    pub fn new(top_node: EvalObject, layout_tree: Arc<Mutex<T>>) -> Self {
        let mut evaluator =
            Evaluator { context: Default::default(), layout_tree, deps_map: Default::default() };
        let top_gen = Arc::new(move |_context| top_node.clone());
        let _root = evaluator.evaluate_unconditional(top_gen, evaluator.context.clone());
        evaluator.context.global.write().update_tree();

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
        let children: Vec<_> = evaluated
            .children
            .into_iter()
            .map(|(key_part, gen)| {
                let context = context.with_key_widget(context.widget_local.key.with(key_part));
                self.evaluate_unconditional(gen, context)
            })
            .collect();

        if evaluated.layout_object.is_some() || (key == Default::default()) {
            let mut layout_tree = self.layout_tree.lock();
            if let Some(layout_object) = evaluated.layout_object.clone() {
                layout_tree.set_node(key, layout_object);
            }
            layout_tree
                .set_children(key, Self::get_layout_children(&mut children.iter()).into_iter());
        }

        let to_return = Arc::new(RefCell::new(EvaluatedEvalObject {
            key,
            gen,
            deps: deps.clone(),
            children,
            layout_object: evaluated.layout_object,
        }));
        for key in deps {
            self.deps_map.entry(key).or_default().push(to_return.clone());
        }
        to_return
    }

    pub fn update(&mut self) -> bool {
        for i in 0..32 {
            if !self.update_once() {
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

        if let Some(layout_object) = frag.layout_object.clone() {
            let mut layout_tree = self.layout_tree.lock();
            layout_tree.set_node(frag.key, layout_object);

            let layout_children = Self::get_layout_children(&mut frag.children.iter());
            layout_tree.set_children(frag.key, layout_children.into_iter());
        }

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

    fn get_layout_children(
        children: &mut dyn Iterator<Item = &Arc<RefCell<EvaluatedEvalObject>>>,
    ) -> Vec<Key> {
        let mut to_return = Vec::new();
        Self::get_layout_children_inner(children, &mut to_return);
        to_return
    }
    fn get_layout_children_inner(
        children: &mut dyn Iterator<Item = &Arc<RefCell<EvaluatedEvalObject>>>,
        vec: &mut Vec<Key>,
    ) {
        for child in children {
            let child = child.borrow();
            if child.layout_object.is_some() {
                vec.push(child.key);
            } else {
                Self::get_layout_children_inner(&mut child.children.iter(), vec)
            }
        }
    }
}
