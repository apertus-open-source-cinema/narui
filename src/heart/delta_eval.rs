// do partial re evaluation of the changed widget tree

use crate::{heart::{Context, Fragment, Key, LayoutTree}, FragmentInner, RenderObject};
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};
use std::{cell::RefCell, sync::Arc};
use rutter_layout::Layout;

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

    pub layout: Arc<dyn Layout>,
    pub render_object: Option<RenderObject>,
    pub children: Vec<EvaluatedFragment>,
}

// The evaluator outputs nothing but rather communicates with the layouter with
// the LayoutTree trait.
pub struct Evaluator<T: LayoutTree> {
    pub context: Context,
    pub layout_tree: Arc<T>,
    deps_map: HashMap<Key, Vec<Arc<RefCell<EvaluatedFragment>>>>,
}
impl<T: LayoutTree + 'static> Evaluator<T> {
    pub fn new(top_node: Fragment, layout_tree: Arc<T>) -> Self {
        let mut evaluator = Evaluator {
            context: Context::new(layout_tree.clone()),
            layout_tree: layout_tree.clone(),
            deps_map: Default::default()
        };
        let _root = evaluator.evaluate_unconditional(top_node, evaluator.context.clone());
        evaluator.context.global.write().tree.update_tree();

        evaluator
    }

    pub fn update(&mut self) -> bool {
        for i in 0..32 {
            if !self.update_once() {
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
            inner: evaluated,
        }));

        let children_keys: Vec<_> = evaluated
            .iter_children()
            .map(|fragment| self.evaluate_unconditional(fragment, context.clone()).borrow().key)
            .collect();
        Self::check_unique_keys_children(children_keys);

        self.layout_tree.set_node(&fragment.key, evaluated.layout(), evaluated.render_object());
        self.layout_tree.set_children(&fragment.key, &children_keys[..]);

        for key in deps {
            self.deps_map.entry(key).or_default().push(to_return.clone());
        }
        to_return
    }

    fn re_eval_fragment(&mut self, frag_cell: Arc<RefCell<EvaluatedFragment>>) {
        let mut frag = frag_cell.borrow_mut();

        let context = self.context.with_key_widget(frag.key);

        let old_children: Vec<_> = frag.inner.clone().iter_children().collect();

        let evaluated: FragmentInner = (frag.gen)(context.clone());
        frag.inner = evaluated;

        let new_deps = &mut *context.widget_local.used.lock();
        for to_remove in frag.deps.difference(new_deps) {
            self.deps_map.entry(*to_remove).or_default().retain(|x| x.borrow().key != frag.key)
        }
        for to_insert in new_deps.difference(&frag.deps) {
            self.deps_map.entry(*to_insert).or_default().push(frag_cell.clone())
        }
        frag.deps = std::mem::replace(new_deps, HashSet::new());


        let children_keys: Vec<_> = frag.inner.iter_children().map(|child| child.borrow().key).collect();
        Self::check_unique_keys_children(children_keys);

        self.layout_tree.set_node(&frag.key, frag.inner.layout(), frag.inner.render_object());
        self.layout_tree.set_children(&frag.key, &children_keys);

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

    fn check_unique_keys_children(children_keys: Vec<Key>) {
        let has_duplicates = (1..children_keys.len()).any(|i| children_keys[i..].contains(&children_keys[i - 1]));
        assert!(
            !has_duplicates,
            "elements need to have unique keys but do not. consider passing an explicit key."
        );
    }
}
