/* The API to the UI library mimics React ergonomics with hooks. There are two types of widgets:
Primitive and Composed Widgets. Primitive Widgets are the variants of the `PrimitiveWidget` Enum.
Composed Widgets are functions that return either other Composed Widgets or Primitive Widgets.
For layout we create `TreeNodes` with stretch Style attributes.
*/

use lyon::path::Path;
use lyon::geom::Point;
use stretch::style::Style;


enum RenderObject {
    Path(Path),
    Text {text: String, size: f32},
}

struct PositionedRenderObject {
    widget: RenderObject,
    position: Point<f32>,
    z_index: i32,
}

enum TreeChildren {
    Children(Vec<TreeNode>),
    Leaf(RenderObject)
}

struct TreeNode {
    pub style: Style,
    pub children: TreeChildren
}
