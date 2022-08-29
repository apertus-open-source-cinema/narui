use crate::{
    context::{
        context::{
            AfterFrameCallback,
            FragmentStore,
            MaybeEvaluatedFragment::{Evaluated, Unevaluated},
            VulkanContext,
        },
        key::KeyMap,
        patched_tree::PatchedTree,
    },
    eval::{
        fragment::UnevaluatedFragment,
        layout::{LayoutTree, Layouter, ScaleFactor},
    },
    CallbackContext,
    Fragment,
    FragmentChildren,
    FragmentInner,
    Key,
    WidgetContext,
};
use derivative::Derivative;
use freelist::Idx;
use hashbrown::HashSet;
use std::sync::Arc;

/// EvaluatedEvalObject is analog to a EvalObject but not lazy and additionally
/// contains the dependencies of Node for allowing partial rebuild.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct EvaluatedFragment {
    // these fields are part of the original Fragment
    pub key: Key,
    #[derivative(Debug = "ignore")]
    pub gen: Option<Box<dyn Fn(&mut WidgetContext) -> FragmentInner>>,

    // this field is information that is gathered by the delta evaluator
    pub layout_idx: Idx,
    pub store_idx: Fragment,

    pub children: FragmentChildren,
}

pub struct EvaluatorInner {
    vulkan_context: VulkanContext,
    pub(crate) tree: Arc<PatchedTree>,
}

impl EvaluatorInner {
    fn update(
        &mut self,
        layout_tree: &mut Layouter,
        fragment_store: &mut FragmentStore,
        after_frame_callbacks: &mut Vec<AfterFrameCallback>,
        key_map: &mut KeyMap,
    ) -> bool {
        let mut to_update: HashSet<Fragment, ahash::RandomState> = HashSet::default();

        let touched_keys = self.tree.update_tree(key_map);
        for key in touched_keys {
            // println!("touched key ({:?}, {})", key_map.key_debug(key.0), key.1);
            for frag in self.tree.dependents(key) {
                to_update.insert(frag);
            }
        }

        if to_update.is_empty() {
            return false;
        }

        for idx in to_update.drain() {
            self.re_eval_fragment(layout_tree, fragment_store, after_frame_callbacks, key_map, idx)
        }

        loop {
            let mut len = 0;
            for idx in fragment_store.dirty_args().collect::<Vec<_>>() {
                len += 1;
                self.re_eval_fragment(
                    layout_tree,
                    fragment_store,
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
        log::trace!(
            "unconditionally evaluating {:?}",
            context.key_map.key_debug(context.fragment_store.get(fragment_idx).key())
        );
        let (layout_idx, children, gen) = {
            let (key, gen) = {
                let UnevaluatedFragment { key, gen } =
                    context.fragment_store.get_mut(fragment_idx).assert_unevaluated_mut();
                (*key, gen.take().unwrap())
            };
            let mut context = context.with_key_widget(key, fragment_idx);
            let evaluated: FragmentInner = gen(&mut context);

            let (layout, render_object, children, is_clipper, subpass) = evaluated.unpack();
            let layout_idx =
                layout_tree.add_node(layout, render_object, is_clipper, subpass, fragment_idx);

            for child in &children {
                log::trace!("evaluating (unconditionally) child {:?} of {:?}", child, fragment_idx);
                self.evaluate_unconditional(*child, layout_tree, &mut context);
            }

            layout_tree.set_children(
                layout_idx,
                children
                    .iter()
                    .map(|idx| context.fragment_store.get(*idx).assert_evaluated().layout_idx),
            );
            (layout_idx, children, gen)
        };

        take_mut::take(context.fragment_store.get_mut(fragment_idx), |unevaluated| {
            let frag = unevaluated.into_unevaluated();
            Evaluated(EvaluatedFragment {
                layout_idx,
                key: frag.key,
                gen: Some(gen),
                store_idx: fragment_idx,
                children,
            })
        });

        log::trace!("finished evaluating {:?}", fragment_idx);

        fragment_idx
    }

    fn re_eval_fragment(
        &mut self,
        layout_tree: &mut Layouter,
        fragment_store: &mut FragmentStore,
        after_frame_callbacks: &mut Vec<AfterFrameCallback>,
        key_map: &mut KeyMap,
        frag_idx: Fragment,
    ) {
        if unsafe { fragment_store.removed(frag_idx) } {
            log::trace!("tried to reeval already removed fragment {:?}, skipping it", frag_idx);
            return;
        }

        match fragment_store.get_mut(frag_idx) {
            Unevaluated(UnevaluatedFragment { key, .. }) => {
                let key = *key;
                log::trace!("tried to reeval unevaluated fragment: {:?}", key_map.key_debug(key));
                self.evaluate_unconditional(
                    frag_idx,
                    layout_tree,
                    &mut WidgetContext::for_fragment(
                        self.vulkan_context.clone(),
                        self.tree.clone(),
                        fragment_store,
                        key,
                        frag_idx,
                        after_frame_callbacks,
                        key_map,
                    ),
                );
            }
            Evaluated(EvaluatedFragment { key, gen, .. }) => {
                log::trace!("reevaluating {:?}", key_map.key_debug(*key));
                let key = *key;
                let gen = gen.take().unwrap();

                let evaluated = {
                    let mut context = WidgetContext::for_fragment(
                        self.vulkan_context.clone(),
                        self.tree.clone(),
                        fragment_store,
                        key,
                        frag_idx,
                        after_frame_callbacks,
                        key_map,
                    );

                    let evaluated: FragmentInner = (gen)(&mut context);
                    evaluated
                };
                let (layout, render_object, children, is_clipper, subpass) = evaluated.unpack();

                let (old_children, layout_idx) = {
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
                                    self.vulkan_context.clone(),
                                    self.tree.clone(),
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
                        children
                            .iter()
                            .map(|c| fragment_store.get(*c).assert_evaluated().layout_idx),
                    );
                }

                let old_children = {
                    let frag = fragment_store.get_mut(frag_idx).assert_evaluated_mut();

                    frag.gen = Some(gen);
                    frag.children = children;

                    layout_tree.set_node(
                        frag.layout_idx,
                        layout,
                        render_object,
                        is_clipper,
                        subpass,
                        frag_idx,
                    );

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
        let EvaluatedFragment { key, children, layout_idx, store_idx, .. } =
            fragment_store.get(frag).assert_evaluated();
        let key = *key;
        let layout_idx = *layout_idx;
        let store_idx = *store_idx;
        for child in children.clone() {
            self.remove_tree(key_map, layout_tree, fragment_store, child);
        }
        self.tree.remove_widget(&key);

        log::trace!("removing layout_node {:?}", key_map.key_debug(key));
        layout_tree.remove_node(layout_idx);
        key_map.remove(&key);

        fragment_store.remove(store_idx);
    }
}

// The evaluator outputs nothing but rather communicates with the layouter with
// the LayoutTree trait.
pub struct Evaluator {
    pub(crate) key_map: KeyMap,
    pub(crate) after_frame_callbacks: Vec<AfterFrameCallback>,
    fragment_store: FragmentStore,
    inner: EvaluatorInner,
    pub(crate) top_node: Idx,
}

impl Evaluator {
    pub fn new(
        vulkan_context: VulkanContext,
        top_node_frag: UnevaluatedFragment,
        layout_tree: &mut Layouter,
    ) -> Self {
        let mut evaluator =
            Evaluator::empty(vulkan_context.clone(), unsafe { Idx::new_unchecked(1) }); // random dummy value
        let top_node = evaluator.fragment_store.add_empty_fragment();
        evaluator.fragment_store.add_fragment(top_node, || top_node_frag);
        let _root = evaluator.inner.evaluate_unconditional(
            top_node,
            layout_tree,
            &mut WidgetContext::root(
                vulkan_context,
                top_node,
                evaluator.inner.tree.clone(),
                &mut evaluator.fragment_store,
                &mut evaluator.after_frame_callbacks,
                &mut evaluator.key_map,
            ),
        );
        evaluator.top_node = evaluator.fragment_store.get(top_node).assert_evaluated().layout_idx;
        std::mem::drop(evaluator.fragment_store.dirty_args());

        evaluator
    }

    fn empty(vulkan_context: VulkanContext, top_node: Idx) -> Self {
        Self {
            key_map: Default::default(),
            after_frame_callbacks: vec![],
            fragment_store: Default::default(),
            inner: EvaluatorInner { vulkan_context, tree: Arc::new(Default::default()) },
            top_node,
        }
    }

    pub fn update(&mut self, layout_tree: &mut Layouter) -> bool {
        self.inner.update(
            layout_tree,
            &mut self.fragment_store,
            &mut self.after_frame_callbacks,
            &mut self.key_map,
        )
    }

    pub fn callback_context<'a>(
        &'a self,
        layout: &'a Layouter,
        scale_factor: &'a ScaleFactor,
    ) -> CallbackContext<'a> {
        CallbackContext {
            tree: self.inner.tree.clone(),
            layout,
            key_map: &self.key_map,
            fragment_store: &self.fragment_store,
            scale_factor,
        }
    }
}
