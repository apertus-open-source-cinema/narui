use super::{
    key::{Key, KeyMap},
    patched_tree::{HookKey, PatchedTree},
};
use crate::eval::{
    delta_eval::EvaluatedFragment,
    fragment::{Fragment, UnevaluatedFragment},
    layout::Layouter,
};
use derivative::Derivative;
use freelist::FreeList;
use smallvec::SmallVec;
use std::{any::Any, fmt::Debug, sync::Arc};


// Context types
// thread access
//   - get value (not listen because we don't have the rebuild if changed thing)
//   - shout
// widget access
//   - create listenable
//   - listen
//   - create after-frame-callback
// callback access
//   - shout
//   - get value
//   - measure

#[derive(Debug)]
pub enum MaybeEvaluatedFragment {
    Unevaluated(UnevaluatedFragment),
    Evaluated(EvaluatedFragment),
}

impl MaybeEvaluatedFragment {
    pub(crate) fn key(&self) -> Key {
        match self {
            MaybeEvaluatedFragment::Unevaluated(frag) => frag.key,
            MaybeEvaluatedFragment::Evaluated(frag) => frag.key,
        }
    }

    pub(crate) fn assert_evaluated(&self) -> &EvaluatedFragment {
        match self {
            MaybeEvaluatedFragment::Evaluated(frag) => frag,
            _ => panic!("tried to get a evaluated fragment from {:?}, but was unevaluated", self),
        }
    }

    pub(crate) fn assert_unevaluated(&self) -> &UnevaluatedFragment {
        match self {
            MaybeEvaluatedFragment::Unevaluated(frag) => frag,
            _ => panic!("tried to get a unevaluated fragment from {:?}, but was evaluated. This error might occur if you have two children with the same Key", self),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn into_evaluated(self) -> EvaluatedFragment {
        match self {
            MaybeEvaluatedFragment::Evaluated(frag) => frag,
            _ => panic!("tried to get a evaluated fragment from {:?}, but was unevaluated", self),
        }
    }

    pub(crate) fn into_unevaluated(self) -> UnevaluatedFragment {
        match self {
            MaybeEvaluatedFragment::Unevaluated(frag) => frag,
            _ => panic!("tried to get a unevaluated fragment from {:?}, but was evaluated", self),
        }
    }

    pub(crate) fn assert_evaluated_mut(&mut self) -> &mut EvaluatedFragment {
        match self {
            MaybeEvaluatedFragment::Evaluated(frag) => frag,
            _ => panic!("tried to get a evaluated fragment from {:?}, but was unevaluated", self),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn assert_unevaluated_mut(&mut self) -> &mut UnevaluatedFragment {
        match self {
            MaybeEvaluatedFragment::Unevaluated(frag) => frag,
            _ => panic!("tried to get a unevaluated fragment from {:?}, but was evaluated", self),
        }
    }
}

#[derive(Debug)]
pub struct FragmentInfo {
    pub fragment: Option<MaybeEvaluatedFragment>,
    pub args: Option<SmallVec<[Box<dyn Any>; 8]>>,
    pub external_hook_count: u16,
}

#[derive(Debug, Default)]
pub struct FragmentStore {
    pub(crate) data: FreeList<FragmentInfo>,
    dirty_args: Vec<Fragment>,
}

impl FragmentStore {
    pub fn next_external_hook_count(&mut self, idx: Fragment) -> u16 {
        let count = self.data[idx.0].external_hook_count;
        self.data[idx.0].external_hook_count += 1;
        count
    }

    pub fn reset_external_hook_count(&mut self, idx: Fragment) {
        self.data[idx.0].external_hook_count = 0;
    }

    pub fn add_empty_fragment(&mut self) -> Fragment {
        let idx = Fragment(self.data.add(FragmentInfo {
            fragment: None,
            args: None,
            external_hook_count: 0,
        }));
        log::trace!("initialized a new fragment with idx {:?}", idx);
        idx
    }

    pub unsafe fn removed(&mut self, idx: Fragment) -> bool {
        self.data.removed(idx.0) || self.data[idx.0].fragment.is_none()
    }

    pub fn add_fragment(
        &mut self,
        idx: Fragment,
        init: impl FnOnce() -> UnevaluatedFragment,
    ) -> Fragment {
        if self.data[idx.0].fragment.is_none() {
            log::trace!("adding fragment {:?}", idx);
            self.data[idx.0].fragment = Some(MaybeEvaluatedFragment::Unevaluated(init()));
        }
        idx
    }

    pub(crate) fn get(&self, idx: Fragment) -> &MaybeEvaluatedFragment {
        self.data[idx.0].fragment.as_ref().unwrap()
    }

    pub(crate) fn get_mut(&mut self, idx: Fragment) -> &mut MaybeEvaluatedFragment {
        self.data[idx.0].fragment.as_mut().unwrap()
    }

    pub fn remove(&mut self, idx: Fragment) {
        self.data[idx.0].fragment = None;
        self.data.remove(idx.0);
    }

    pub fn get_args(&self, idx: Fragment) -> &Option<SmallVec<[Box<dyn Any>; 8]>> {
        &self.data[idx.0].args
    }

    pub fn set_args(&mut self, idx: Fragment, args: SmallVec<[Box<dyn Any>; 8]>) {
        self.dirty_args.push(idx);
        self.data[idx.0].args = Some(args);
    }

    pub fn dirty_args<'a>(&'a mut self) -> impl Iterator<Item = Fragment> + 'a {
        self.dirty_args.drain(..).rev()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WidgetContext<'a> {
    pub widget_local: WidgetLocalContext,
    #[derivative(Debug = "ignore")]
    pub tree: Arc<PatchedTree>,
    pub local_hook: bool,
    pub fragment_store: &'a mut FragmentStore,
    pub widget_loc: (usize, usize),
    #[derivative(Debug(format_with = "crate::util::format_helpers::print_vec_len"))]
    pub(crate) after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
    pub key_map: &'a mut KeyMap,
}

impl<'a> WidgetContext<'a> {
    pub fn key_for_hook(&mut self) -> HookKey {
        if self.local_hook {
            let counter = self.widget_local.hook_counter;
            self.widget_local.hook_counter += 1;
            log::trace!(
                "creating local hook: {:?}:{}",
                self.key_map.key_debug(self.widget_local.key),
                counter
            );
            (self.widget_local.key, counter)
        } else {
            let key = self.fragment_store.next_external_hook_count(self.widget_local.idx)
                | 0b1000_0000_0000_0000;
            log::trace!(
                "creating external hook: {:?}:{}",
                self.key_map.key_debug(self.widget_local.key),
                key
            );
            (self.widget_local.key, key)
        }
    }

    pub fn thread_context(&self) -> ThreadContext { ThreadContext { tree: self.tree.clone() } }

    pub fn root(
        top: Fragment,
        tree: Arc<PatchedTree>,
        fragment_store: &'a mut FragmentStore,
        after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
        key_map: &'a mut KeyMap,
    ) -> Self {
        Self {
            tree,
            after_frame_callbacks,
            fragment_store,
            widget_local: WidgetLocalContext::for_key(Default::default(), top),
            widget_loc: (0, 0),
            key_map,
            local_hook: true,
        }
    }

    pub fn for_fragment(
        tree: Arc<PatchedTree>,
        fragment_store: &'a mut FragmentStore,
        key: Key,
        idx: Fragment,
        after_frame_callbacks: &'a mut Vec<AfterFrameCallback>,
        key_map: &'a mut KeyMap,
    ) -> Self {
        WidgetContext {
            tree,
            after_frame_callbacks,
            fragment_store,
            widget_local: WidgetLocalContext::for_key(key, idx),
            widget_loc: (0, 0),
            key_map,
            local_hook: true,
        }
    }

    pub fn with_key_widget(&mut self, key: Key, idx: Fragment) -> WidgetContext {
        WidgetContext {
            tree: self.tree.clone(),
            local_hook: true,
            fragment_store: self.fragment_store,
            widget_loc: (0, 0),
            after_frame_callbacks: self.after_frame_callbacks,
            widget_local: WidgetLocalContext::for_key(key, idx),
            key_map: &mut self.key_map,
        }
    }
}

#[derive(Clone)]
pub struct ThreadContext {
    pub(crate) tree: Arc<PatchedTree>,
}

pub struct CallbackContext<'a> {
    pub(crate) tree: Arc<PatchedTree>,
    pub key_map: &'a KeyMap,
    pub(crate) layout: &'a Layouter,
    pub(crate) fragment_store: &'a FragmentStore,
}


pub type AfterFrameCallback = Box<dyn for<'a> Fn(&'a CallbackContext<'a>)>;

#[derive(Clone, Debug)]
pub struct WidgetLocalContext {
    pub key: Key,
    pub idx: Fragment,
    pub hook_counter: u16,
}

impl WidgetLocalContext {
    pub fn for_key(key: Key, idx: Fragment) -> Self { Self { idx, key, hook_counter: 0 } }
}
