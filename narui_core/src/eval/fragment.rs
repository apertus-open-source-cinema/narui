use crate::{
    util::geom::{Rect, Vec2},
    vulkano_render::lyon::ColoredBuffersBuilder,
    CallbackContext,
    Color,
    Dimension,
    Key,
    WidgetContext,
};
use derivative::Derivative;
use freelist::Idx;
use rutter_layout::Layout;
use smallvec::{smallvec, SmallVec};
use std::{rc::Rc, sync::Arc};
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
    render_pass::RenderPass,
};

/*
The general flow of a frame in narui:
Evaluation -> Layout -> Rendering

1. Evaluation
the output of this Stage is a tree of LayoutObjects

2. Layout
the outputs of this stage are PositionedRenderObjects

3. Rendering
the output of this stage is the visual output :). profit!

 */

/// Fragment is merely a reference (for performance reasons)
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Fragment(pub(crate) u32);
impl From<Key> for Fragment {
    fn from(key: Key) -> Self { Fragment(key.0) }
}
impl From<Fragment> for Idx {
    fn from(fragment: Fragment) -> Self { unsafe { Idx::new_unchecked(fragment.0 as _) } }
}
impl From<Fragment> for Key {
    fn from(fragment: Fragment) -> Self { Key(fragment.0) }
}
impl From<Fragment> for FragmentChildren {
    fn from(fragment: Fragment) -> Self { smallvec![fragment] }
}

pub type FragmentChildren = SmallVec<[Fragment; 8]>;


#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct UnevaluatedFragment {
    pub key: Key,
    #[derivative(Debug = "ignore")]
    pub gen: Rc<dyn Fn(&mut WidgetContext) -> FragmentInner>,
}
impl PartialEq for UnevaluatedFragment {
    fn eq(&self, other: &Self) -> bool { self.key == other.key }
}

#[derive(Debug)]
pub enum FragmentInner {
    Leaf { render_object: RenderObject, layout: Box<dyn Layout> },
    Node { children: FragmentChildren, layout: Box<dyn Layout>, is_clipper: bool },
}
impl FragmentInner {
    pub fn unpack(self) -> (Box<dyn Layout>, Option<RenderObject>, FragmentChildren, bool) {
        match self {
            Self::Leaf { render_object, layout } => {
                (layout, Some(render_object), SmallVec::new(), false)
            }
            Self::Node { children, layout, is_clipper } => (layout, None, children, is_clipper),
        }
    }

    pub fn from_fragment(fragment: Fragment) -> Self {
        FragmentInner::Node {
            children: smallvec![fragment],
            layout: Box::new(rutter_layout::layouts::Transparent),
            is_clipper: false,
        }
    }
}

pub type PathGenInner = dyn Fn(
    Vec2, // size
    &mut lyon::tessellation::FillTessellator,
    &mut lyon::tessellation::StrokeTessellator,
    ColoredBuffersBuilder,
);
pub type RenderFnInner = dyn Fn(
    Arc<RenderPass>,
    &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    &DynamicState,
    Rect, // target rect
    Vec2, // window dimensions
);
/// RenderObject is the data structure that really defines _what_ is rendered
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderObject {
    DebugRect,
    RoundedRect {
        inverted: bool,
        stroke_color: Option<Color>,
        fill_color: Option<Color>,
        stroke_width: f32,
        border_radius: Dimension,
    },
    Path {
        #[derivative(Debug = "ignore")]
        path_gen: Arc<PathGenInner>,
    },
    Text {
        key: Key,
        text: Rc<String>,
        size: f32,
        color: Color,
    },
    Input {
        key: Key,
        // this is nothing that gets rendered but instead it gets interpreted by the input handling
        // logic
        #[derivative(Debug = "ignore")]
        on_click: Arc<dyn Fn(&CallbackContext, bool)>,
        #[derivative(Debug = "ignore")]
        on_hover: Arc<dyn Fn(&CallbackContext, bool)>,
        #[derivative(Debug = "ignore")]
        on_move: Arc<dyn Fn(&CallbackContext, Vec2)>,
    },
    Raw {
        #[derivative(Debug = "ignore")]
        render_fn: Arc<RenderFnInner>,
    },
    None,
}
