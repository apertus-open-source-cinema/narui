// do partial re evaluation of the changed widget tree

use crate::{
    heart::{Fragment, Key, LayoutTree},
    AfterFrameCallback,
    ArgsTree,
    CallbackContext,
    FragmentInner,
    KeyPart,
    Layouter,
    PatchedTree,
    RenderObject,
    WidgetContext,
    WidgetLocalContext,
};
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};
use rutter_layout::Layout;
use std::{cell::RefCell, rc::Rc, sync::Arc};

// EvaluatedEvalObject is analog to a EvalObject but not lazy and additionally
// contains the dependencies of Node for allowing partial rebuild.
#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct EvaluatedFragment {
    // these fields are part of the original Fragment
    pub key: Key,
    #[derivative(Debug = "ignore")]
    pub gen: Rc<dyn Fn(&mut WidgetContext) -> FragmentInner>,

    // this field is information that is gathered by the delta evaluator
    pub deps: HashSet<Key, ahash::RandomState>,

    pub children: Vec<Rc<RefCell<EvaluatedFragment>>>,
}

#[derive(Default)]
pub struct EvaluatorInner {
    pub(crate) tree: Arc<PatchedTree>,
    deps_map: HashMap<
        Key,
        HashMap<Key, Rc<RefCell<EvaluatedFragment>>, ahash::RandomState>,
        ahash::RandomState,
    >,
    key_to_fragment: HashMap<Key, Rc<RefCell<EvaluatedFragment>>, ahash::RandomState>,
}

impl EvaluatorInner {
    fn update(
        &mut self,
        layout_tree: &mut Layouter,
        args_tree: &mut ArgsTree,
        after_frame_callbacks: &mut Vec<AfterFrameCallback>,
    ) -> bool {
        let mut to_update: HashMap<Key, Rc<RefCell<EvaluatedFragment>>, ahash::RandomState> =
            HashMap::default();

        let touched_keys = self.tree.update_tree();
        for key in touched_keys.into_iter() {
            for frag in self.deps_map.get(&key).into_iter().flat_map(|v| v.values()) {
                to_update.entry(frag.borrow().key).or_insert_with(|| frag.clone());
            }
        }

        if to_update.is_empty() {
            return false;
        }

        loop {
            let touched_args = args_tree.dirty();
            for key in touched_args {
                to_update.entry(key.clone()).or_insert_with(|| self.key_to_fragment[&key].clone());
            }

            if to_update.len() == 0 {
                return true;
            }
            for (_, frag) in to_update.drain() {
                self.re_eval_fragment(layout_tree, args_tree, after_frame_callbacks, frag.clone())
            }
        }
    }

    fn evaluate_unconditional(
        &mut self,
        fragment: Fragment,
        layout_tree: &mut Layouter,
        context: &mut WidgetContext,
    ) -> Rc<RefCell<EvaluatedFragment>> {
        let mut context = context.with_key_widget(fragment.key);
        let evaluated: FragmentInner = (fragment.gen)(&mut context);
        let mut deps = std::mem::replace(&mut context.widget_local.used, HashSet::default());

        let (layout, render_object, children) = evaluated.unpack();

        let (children_keys, children): (Vec<_>, Vec<_>) = children
            .map(|fragment| {
                let evaluated = self.evaluate_unconditional(fragment, layout_tree, &mut context);
                let key = evaluated.borrow().key;
                (key, evaluated)
            })
            .unzip();

        let to_return = Rc::new(RefCell::new(EvaluatedFragment {
            key: fragment.key,
            gen: fragment.gen,
            deps: deps.clone(),
            children,
        }));

        Self::check_unique_keys_children(children_keys.iter());

        layout_tree.set_node(&fragment.key, layout, render_object);
        layout_tree.set_children(&fragment.key, &children_keys[..]);

        for key in deps {
            self.deps_map.entry(key).or_default().insert(to_return.borrow().key, to_return.clone());
        }
        self.key_to_fragment.insert(fragment.key, to_return.clone());

        to_return
    }

    fn re_eval_fragment(
        &mut self,
        layout_tree: &mut Layouter,
        args_tree: &mut ArgsTree,
        after_frame_callbacks: &mut Vec<AfterFrameCallback>,
        frag_cell: Rc<RefCell<EvaluatedFragment>>,
    ) {
        let mut frag = frag_cell.borrow_mut();

        let mut context = WidgetContext::for_fragment(
            self.tree.clone(),
            args_tree,
            frag.key,
            after_frame_callbacks,
        );

        let evaluated: FragmentInner = (frag.gen)(&mut context);
        let (layout, render_object, children) = evaluated.unpack();

        let mut old_children = std::mem::replace(&mut frag.children, vec![]);

        let new_deps = &mut context.widget_local.used;
        for to_remove in frag.deps.difference(new_deps) {
            self.deps_map.entry(*to_remove).or_default().remove(&frag.key);
        }

        for to_insert in new_deps.difference(&frag.deps) {
            self.deps_map.entry(*to_insert).or_default().insert(frag.key, frag_cell.clone());
        }
        frag.deps = std::mem::replace(new_deps, HashSet::default());


        let mut children_keys = vec![];
        for child in children {
            children_keys.push(child.key);

            let evaluated = match old_children
                .iter()
                .position(|frag| frag.borrow().key == child.key)
                .map(|idx| old_children.swap_remove(idx))
            {
                Some(f) => f,
                None => self.evaluate_unconditional(child, layout_tree, &mut context),
            };

            frag.children.push(evaluated);
        }


        Self::check_unique_keys_children(children_keys.iter());
        layout_tree.set_node(&frag.key, layout, render_object);
        layout_tree.set_children(&frag.key, &children_keys);

        for child in old_children {
            self.remove_tree(layout_tree, args_tree, &*child);
            self.tree.remove(frag.key);
            args_tree.remove(frag.key);
        }
    }

    fn remove_tree(
        &mut self,
        layout_tree: &mut Layouter,
        args_tree: &mut ArgsTree,
        tree: &RefCell<EvaluatedFragment>,
    ) {
        let frag = tree.borrow();
        for child in frag.children.iter() {
            self.remove_tree(layout_tree, args_tree, child);
        }
        for to_remove in frag.deps.iter() {
            self.deps_map.entry(*to_remove).or_default().remove(&frag.key);
        }
        layout_tree.remove_node(&frag.key);
    }

    fn check_unique_keys_children<'k>(children_keys: impl Iterator<Item = &'k Key>) {
        let mut keys = HashSet::new();
        for key in children_keys {
            if keys.contains(key) {
                panic!(
                    "elements need to have unique keys but do not. consider passing an explicit key."
                );
            } else {
                keys.insert(key.clone());
            }
        }
    }
}

// The evaluator outputs nothing but rather communicates with the layouter with
// the LayoutTree trait.
#[derive(Default)]
pub struct Evaluator {
    args_tree: ArgsTree,
    pub after_frame_callbacks: Vec<AfterFrameCallback>,
    inner: EvaluatorInner,
}

impl Evaluator {
    pub fn new(top_node: Fragment, layout_tree: &mut Layouter) -> Self {
        let mut evaluator = Evaluator::default();
        let _root = evaluator.inner.evaluate_unconditional(
            top_node,
            layout_tree,
            &mut WidgetContext::root(
                evaluator.inner.tree.clone(),
                &mut evaluator.args_tree,
                &mut evaluator.after_frame_callbacks,
            ),
        );
        evaluator.inner.tree.update_tree();

        evaluator
    }

    pub fn update(&mut self, layout_tree: &mut Layouter) -> bool {
        self.inner.update(layout_tree, &mut self.args_tree, &mut self.after_frame_callbacks)
    }

    pub fn callback_context<'layout>(&self, layout: &'layout Layouter) -> CallbackContext<'layout> {
        CallbackContext { tree: self.inner.tree.clone(), layout }
    }
}
