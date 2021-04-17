/* The API to the UI library mimics React ergonomics with hooks. There are two types of widgets:
Primitive and Composed Widgets. Primitive Widgets are the variants of the `PrimitiveWidget` Enum.
Composed Widgets are functions that return either other Composed Widgets or Primitive Widgets.
For layout we create `TreeNodes` with stretch Style attributes.
*/

use lyon::{geom::Point, path::Path};
use stretch::style::Style;

#[derive(Debug)]
pub struct Widget {
    pub style: Style,
    pub children: TreeChildren,
}
#[derive(Debug)]
pub enum RenderObject {
    Path(Path),
    Text { text: String, size: f32 },
    InputSurface, /* this is nothing that gets rendered but instead it gets interpreted by the
                   * input handling logic */
}
#[derive(Debug)]
pub enum TreeChildren {
    Children(Vec<Widget>),
    Leaf(RenderObject),
}
