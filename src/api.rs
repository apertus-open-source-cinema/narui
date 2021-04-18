/* The API to the UI library mimics React ergonomics with hooks. There are two types of widgets:
Primitive and Composed Widgets. Primitive Widgets are the variants of the `PrimitiveWidget` Enum.
Composed Widgets are functions that return either other Composed Widgets or Primitive Widgets.
For layout we create `TreeNodes` with stretch Style attributes.
*/

use derivative::Derivative;
use lyon::path::Path;
use std::sync::Arc;
use stretch::{geometry::Size, style::Style};

#[derive(Debug, Clone)]
pub struct Widget {
    pub style: Style,
    pub children: Children,
}
impl Into<Vec<Widget>> for Widget {
    fn into(self) -> Vec<Widget> { vec![self] }
}
#[derive(Debug, Clone)]
pub enum Children {
    Composed(Vec<Widget>),
    RenderObject(RenderObject),
}
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderObject {
    Path(#[derivative(Debug = "ignore")] Arc<dyn Fn(Size<f32>) -> Path>),
    Text { text: String, size: f32 },
    InputSurface, /* this is nothing that gets rendered but instead it gets interpreted by the
                   * input handling logic */
}
