/* The API to the UI library mimics React ergonomics with hooks. There are two types of widgets:
Primitive and Composed Widgets. Primitive Widgets are the variants of the `PrimitiveWidget` Enum.
Composed Widgets are functions that return either other Composed Widgets or Primitive Widgets.
For layout we create `TreeNodes` with stretch Style attributes.
*/

use crate::{heart::*, hooks::Listenable};
use derivative::Derivative;
use lyon::{path::Path, tessellation::StrokeOptions};
use std::sync::Arc;
use stretch::{geometry::Size, number::Number, style::Style};
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


pub type Fragment = EvalObject;
// The data structure that is input into the Evaluator Pass. When a EvalObject
// has both a layout_object and children, the children are the children of the
// LayoutObject
#[derive(Clone, Default, Derivative)]
#[derivative(Debug)]
pub struct EvalObject {
    pub key: Key,
    #[derivative(Debug = "ignore")]
    pub children: Vec<(KeyPart, Arc<dyn Fn(Context) -> EvalObject + Send + Sync>)>,
    pub layout_object: Option<LayoutObject>,
}
impl From<EvalObject> for Vec<(KeyPart, Arc<dyn Fn(Context) -> EvalObject + Send + Sync>)> {
    fn from(eval_obj: EvalObject) -> Self {
        vec![(KeyPart::Nop, Arc::new(move |_context| eval_obj.clone()))]
    }
}
pub trait ToFragment {
    fn to_fragment(self, context: Context) -> EvalObject;
}
impl<T: IntoIterator<Item = EvalObject>> ToFragment for T {
    fn to_fragment(self, context: Context) -> EvalObject {
        EvalObject {
            key: context.widget_local.key,
            children: self
                .into_iter()
                .map(|x| (x.key.last_part(), Arc::new(move |_context| x.clone()) as _))
                .collect(),
            layout_object: None,
        }
    }
}
impl PartialEq for EvalObject {
    fn eq(&self, other: &Self) -> bool { self.key == other.key }
}

// A part of the layout tree additionally containing information to render the
// object A LayoutObject is analog to a stretch Node
// but additionally contains a list of RenderObject that can then be passed
// to the render stage.
#[derive(Derivative, Clone, Default)]
#[derivative(Debug)]
pub struct LayoutObject {
    pub style: Style,
    #[derivative(Debug = "ignore")]
    pub measure_function: Option<Arc<dyn Fn(Size<Number>) -> Size<f32> + Send + Sync>>,
    pub render_objects: Vec<(KeyPart, RenderObject)>,
}

pub type PathGenInner = dyn (Fn(Size<f32>) -> Path) + Send + Sync;
pub type PathGen = Listenable<Arc<PathGenInner>>;
// RenderObject is the data structure that really defines _what_ is rendered
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderObject {
    DebugRect,
    FillPath {
        #[derivative(Debug = "ignore")]
        path_gen: PathGen,
        color: Color,
    },
    StrokePath {
        #[derivative(Debug = "ignore")]
        path_gen: PathGen,
        color: Color,
        stroke_options: StrokeOptions,
    },
    Text {
        text: String,
        size: f32,
        color: Color,
    },
    Input {
        // this is nothing that gets rendered but instead it gets interpreted by the input handling
        // logic
        #[derivative(Debug = "ignore")]
        on_click: Arc<dyn Fn(Context, bool) + Send + Sync>,
        #[derivative(Debug = "ignore")]
        on_hover: Arc<dyn Fn(Context, bool) + Send + Sync>,
        #[derivative(Debug = "ignore")]
        on_move: Arc<dyn Fn(Context, Vec2) + Send + Sync>,
    },
    Raw {
        #[derivative(Debug = "ignore")]
        render_fn: Arc<
            dyn Fn(
                    Arc<RenderPass>,
                    &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
                    &DynamicState,
                    Rect,
                ) + Send
                + Sync,
        >,
    },
}
