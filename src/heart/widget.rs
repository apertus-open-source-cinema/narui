/* The API to the UI library mimics React ergonomics with hooks. There are two types of widgets:
Primitive and Composed Widgets. Primitive Widgets are the variants of the `PrimitiveWidget` Enum.
Composed Widgets are functions that return either other Composed Widgets or Primitive Widgets.
For layout we create `TreeNodes` with stretch Style attributes.
*/

use crate::heart::*;
use derivative::Derivative;
use lyon::path::Path;
use std::sync::Arc;
use stretch::{geometry::Size, node::MeasureFunc, style::Style};

#[derive(Derivative)]
#[derivative(Debug)]
pub enum Widget {
    Node {
        style: Style,
        children: Vec<Widget>,
        render_objects: Vec<RenderObject>,
    },
    Leaf {
        style: Style,
        #[derivative(Debug = "ignore")]
        measure_function: MeasureFunc,
        render_objects: Vec<RenderObject>,
    },
}
impl Widget {
    pub fn render_object(render_object: RenderObject, children: Vec<Widget>, style: Style) -> Self {
        Widget::Node { style, children, render_objects: vec![render_object] }
    }
    pub fn layout_block(style: Style, children: Vec<Widget>) -> Self {
        Widget::Node { style, children, render_objects: vec![] }
    }
}
impl Into<Vec<Widget>> for Widget {
    fn into(self) -> Vec<Widget> { vec![self] }
}
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderObject {
    Path {
        #[derivative(Debug = "ignore")]
        path_gen: Arc<dyn Fn(Size<f32>) -> Path>,
        color: Color,
    },
    Text {
        text: String,
        size: f32,
        color: Color,
    },
    Input {
        // this is nothing that gets rendered but instead it gets interpreted by the input handling
        // logic
        hover: StateValue<bool>,
        click: StateValue<bool>,
        position: StateValue<Option<Vec2>>,
    },
}
