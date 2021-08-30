// do partial re evaluation of the changed widget tree

use crate::{heart::{Key, LayoutTree, UnevaluatedFragment}, AfterFrameCallback, CallbackContext, ExternalHookCount, FragmentInner, FragmentStore, HookKey, KeyMap, Layouter, PatchedTree, WidgetContext, Fragment, FragmentChildren, MaybeEvaluatedFragment};
use derivative::Derivative;
use hashbrown::{HashMap, HashSet};

use crate::MaybeEvaluatedFragment::{Evaluated, Unevaluated};
use rutter_layout::Idx;
use smallvec::SmallVec;
use std::{cell::RefCell, fmt::Debug, rc::Rc, sync::Arc};

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
    pub layout_idx: rutter_layout::Idx,
    pub store_idx: Fragment,
    pub deps: HashSet<HookKey, ahash::RandomState>,

    pub children: FragmentChildren,
}

#[derive(Default)]
pub struct EvaluatorInner {
    pub(crate) tree: Arc<PatchedTree>,
    deps_map: HashMap<HookKey, HashSet<Fragment, ahash::RandomState>, ahash::RandomState>,
}

impl EvaluatorInner {
    fn update(
        &mut self,
        layout_tree: &mut Layouter,
        fragment_store: &mut FragmentStore,
        after_frame_callbacks: &mut Vec<AfterFrameCallback>,
        key_map: &mut KeyMap,
    ) -> bool {
        let mut external_hook_count = Default::default();
        let mut to_update: HashSet<Fragment, ahash::RandomState> = HashSet::default();

        let touched_keys = self.tree.update_tree(key_map);
        for key in touched_keys.into_iter() {
            // println!("touched key ({:?}, {})", key_map.key_debug(key.0), key.1);
            for frag in self.deps_map.get(&key).into_iter().flat_map(|v| v.iter()) {
                to_update.insert(*frag);
            }
        }

        if to_update.is_empty() {
            return false;
        }

        for idx in to_update.drain() {
            self.re_eval_fragment(
                layout_tree,
                fragment_store,
                &mut external_hook_count,
                after_frame_callbacks,
                key_map,
                idx,
            )
        }

        loop {
            let mut len = 0;
            for idx in fragment_store.dirty_args().collect::<Vec<_>>() {
                len += 1;
                self.re_eval_fragment(
                    layout_tree,
                    fragment_store,
                    &mut external_hook_count,
                    after_frame_callbacks,
                    key_map,
                    idx,
                )
            }

            if len == 0 {
                return true;
            }
        }
    }

    fn evaluate_unconditional(
        &mut self,
        fragment_idx: Fragment,
        layout_tree: &mut Layouter,
        context: &mut WidgetContext,
    ) -> Fragment {
        log::trace!("unconditionally evaluating {:?}", context.key_map.key_debug(context.fragment_store.get(fragment_idx).key()));
        let (layout_idx, deps, children) = {
            let UnevaluatedFragment { key, gen } =
                context.fragment_store.get(fragment_idx).assert_unevaluated();
            let gen = gen.clone();
            let mut context = context.with_key_widget(*key, fragment_idx);
            let evaluated: FragmentInner = (gen.clone())(&mut context);
            let deps = std::mem::take(&mut context.widget_local.used);

            let (layout, render_object, children, is_clipper) = evaluated.unpack();
            let layout_idx = layout_tree.add_node(layout, render_object, is_clipper);

            for child in &children {
                self.evaluate_unconditional(*child, layout_tree, &mut context);
            }

            layout_tree.set_children(
                layout_idx,
                children
                    .iter()
                    .map(|idx| context.fragment_store.get(*idx).assert_evaluated().layout_idx),
            );
            (layout_idx, deps, children)
        };

        take_mut::take(context.fragment_store.get_mut(fragment_idx), |unevaluated| {
            let frag = unevaluated.into_unevaluated();
            Evaluated(EvaluatedFragment {
                layout_idx,
                key: frag.key,
                gen: frag.gen,
                deps: deps.clone(),
                store_idx: fragment_idx,
                children,
            })
        });

        for key in deps {
            self.deps_map.entry(key).or_default().insert(fragment_idx);
        }

        fragment_idx
    }

    fn re_eval_fragment(
        &mut self,
        layout_tree: &mut Layouter,
        fragment_store: &mut FragmentStore,
        external_hook_count: &mut ExternalHookCount,
        after_frame_callbacks: &mut Vec<AfterFrameCallback>,
        key_map: &mut KeyMap,
        frag_idx: Fragment,
    ) {
        if unsafe { fragment_store.removed(frag_idx) } {
            log::trace!("tried to reeval already removed fragment {:?}, skipping it", frag_idx);
            return;
        }

        match fragment_store.get(frag_idx) {
            Unevaluated(frag) => {
                log::trace!("tried to reeval unevaluated fragment: {:?}", key_map.key_debug(frag.key));
                self.evaluate_unconditional(frag_idx, layout_tree, &mut WidgetContext::for_fragment(self.tree.clone(), external_hook_count, fragment_store, frag.key, frag_idx, after_frame_callbacks, key_map));
            },
            Evaluated(EvaluatedFragment { key, gen, .. }) => {
                log::trace!("reevaluating {:?}", key_map.key_debug(*key));
                let key = key.clone();
                let gen = gen.clone();

                let (mut widget_local, evaluated) = {
                    let mut context = WidgetContext::for_fragment(
                        self.tree.clone(),
                        external_hook_count,
                        fragment_store,
                        key,
                        frag_idx,
                        after_frame_callbacks,
                        key_map,
                    );

                    let evaluated: FragmentInner = (gen)(&mut context);
                    let WidgetContext { widget_local, .. } = context;
                    (widget_local, evaluated)
                };
                let (layout, render_object, children, is_clipper) = evaluated.unpack();

                let (mut old_children, layout_idx) = {
                    let frag = &mut fragment_store.get_mut(frag_idx).assert_evaluated_mut();
                    (&mut std::mem::take(&mut frag.children), frag.layout_idx)
                };
                let num_new_children = children.len();
                let num_old_children = old_children.len();

                for child in &children {
                    match fragment_store.get(*child) {
                        Unevaluated(_) => {
                            self.evaluate_unconditional(
                                *child,
                                layout_tree,
                                &mut WidgetContext::for_fragment(
                                    self.tree.clone(),
                                    external_hook_count,
                                    fragment_store,
                                    key,
                                    *child,
                                    after_frame_callbacks,
                                    key_map,
                                ),
                            );
                        }
                        Evaluated(_) => {
                            old_children
                                .iter()
                                .position(|old_idx| old_idx == child)
                                .map(|idx| old_children.swap_remove(idx));
                        }
                    }
                }

                // if they were both zero nothing changed and we can avoid some unnecessary key
                // lookups
                if (num_old_children != 0) || (num_new_children != 0) {
                    layout_tree.set_children(
                        layout_idx,
                        children.iter().map(|c| fragment_store.get(*c).assert_evaluated().layout_idx),
                    );
                }

                let mut old_children = {
                    let frag = fragment_store.get_mut(frag_idx).assert_evaluated_mut();

                    let new_deps = &mut widget_local.used;
                    for to_remove in frag.deps.difference(new_deps) {
                        self.deps_map.entry(*to_remove).or_default().remove(&frag.store_idx);
                    }

                    for to_insert in new_deps.difference(&frag.deps) {
                        self.deps_map.entry(*to_insert).or_default().insert(frag_idx);
                    }
                    frag.deps = std::mem::take(new_deps);
                    frag.children = children;

                    layout_tree.set_node(frag.layout_idx, layout, render_object, is_clipper);

                    old_children
                };

                for child in old_children {
                    self.remove_tree(key_map, layout_tree, fragment_store, *child);
                }

            }
        }
    }

    fn remove_tree(
        &mut self,
        key_map: &mut KeyMap,
        layout_tree: &mut Layouter,
        fragment_store: &mut FragmentStore,
        frag: Fragment,
    ) {
        let deps = std::mem::take(&mut fragment_store.get_mut(frag).assert_evaluated_mut().deps);
        let EvaluatedFragment { key, children, layout_idx, store_idx, .. } =
            fragment_store.get(frag).assert_evaluated();
        let key = *key;
        let layout_idx = *layout_idx;
        let store_idx = *store_idx;
        for child in children.clone() {
            self.remove_tree(key_map, layout_tree, fragment_store, child);
        }
        for to_remove in deps.iter() {
            self.deps_map.entry(*to_remove).or_default().remove(&store_idx);
        }
        self.tree.remove_widget(&key);

        log::trace!("removing layout_node {:?}", key_map.key_debug(key));
        layout_tree.remove_node(layout_idx);
        key_map.remove(&key);

        fragment_store.remove(key);
    }

    fn check_unique_keys_children(
        parent_debug: impl Debug,
        children_keys: impl Iterator<Item = Key>,
    ) {
        let mut keys = HashSet::new();
        for key in children_keys {
            if keys.contains(&key) {
                panic!(
                    "elements need to have unique keys but children of {:?} do not. consider passing an explicit key.",
                    parent_debug,
                );
            } else {
                keys.insert(key);
            }
        }
    }
}

// The evaluator outputs nothing but rather communicates with the layouter with
// the LayoutTree trait.
#[derive(Derivative)]
#[derivative(Default)]
pub struct Evaluator {
    pub(crate) key_map: KeyMap,
    pub(crate) after_frame_callbacks: Vec<AfterFrameCallback>,
    fragment_store: FragmentStore,
    inner: EvaluatorInner,
    #[derivative(Default(value = "std::num::NonZeroUsize::new(12312803).unwrap()"))]
    pub(crate) top_node: Idx,
}

impl Evaluator {
    pub fn new(top_node_frag: UnevaluatedFragment, layout_tree: &mut Layouter) -> Self {
        let mut evaluator = Evaluator::default();
        let top_node = evaluator.fragment_store.add_empty_fragment(Default::default());
        evaluator.fragment_store.add_fragment(top_node, || top_node_frag);
        let root = evaluator.inner.evaluate_unconditional(
            top_node,
            layout_tree,
            &mut WidgetContext::root(
                top_node,
                evaluator.inner.tree.clone(),
                &mut Default::default(),
                &mut evaluator.fragment_store,
                &mut evaluator.after_frame_callbacks,
                &mut evaluator.key_map,
            ),
        );
        evaluator.top_node = evaluator.fragment_store.get(top_node).assert_evaluated().layout_idx;
        std::mem::drop(evaluator.fragment_store.dirty_args());

        evaluator
    }

    pub fn update(&mut self, layout_tree: &mut Layouter) -> bool {
        self.inner.update(
            layout_tree,
            &mut self.fragment_store,
            &mut self.after_frame_callbacks,
            &mut self.key_map,
        )
    }

    pub fn callback_context<'a>(&'a self, layout: &'a Layouter) -> CallbackContext<'a> {
        CallbackContext {
            tree: self.inner.tree.clone(),
            layout,
            key_map: &self.key_map,
            fragment_store: &self.fragment_store,
        }
    }
}
