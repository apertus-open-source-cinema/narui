/* The layout pass takes the tree where the leaf nodes are Primitive Widgets and converts it to a list
of primitive PositionedPrimitiveWidget s that can then be handled to the rendering Backend.
 */

use crate::api::{LayoutBox, RenderObject, Widget};
use std::{collections::HashMap, ops::Deref};
use stretch::{
    geometry::{Point, Size},
    node::Node,
    number::Number,
    Error,
    Stretch,
};


#[derive(Debug, Clone)]
pub struct PositionedRenderObject {
    pub render_object: RenderObject,
    pub position: Point<f32>,
    pub size: Size<f32>,
    pub z_index: i32,
}

pub fn do_layout(
    top: Widget,
    size: Size<f32>,
) -> Result<Vec<PositionedRenderObject>, stretch::Error> {
    let mut stretch = stretch::node::Stretch::new();
    let mut map = HashMap::new();
    let top_node = if let Widget::LayoutBox(top) = top {
        add_widget_to_stretch(top, &mut stretch, &mut map)?
    } else {
        unimplemented!()
    };

    stretch.compute_layout(
        top_node,
        Size { width: Number::Defined(size.width), height: Number::Defined(size.height) },
    )?;

    let mut to_return = Vec::with_capacity(map.len());
    get_absolute_positions(
        &mut stretch,
        top_node,
        Point { x: 0., y: 0. },
        &mut map,
        &mut to_return,
    );

    print!("{}", print_layout(&stretch, top_node));

    Ok(to_return)
}
fn add_widget_to_stretch(
    layout_box: LayoutBox,
    stretch: &mut Stretch,
    map: &mut HashMap<Node, Vec<RenderObject>>,
) -> Result<Node, Error> {
    let mut render_objects = Vec::new();
    let mut layout_boxes = Vec::new();
    for child in layout_box.children {
        match child {
            Widget::LayoutBox(layout_box) => layout_boxes.push(layout_box),
            Widget::RenderObject(render_object) => render_objects.push(render_object),
        }
    }
    let mut child_nodes = Vec::new();
    for layout_box in layout_boxes {
        child_nodes.push(add_widget_to_stretch(layout_box, stretch, map)?);
    }

    let node = stretch.new_node(layout_box.style, child_nodes)?;
    if render_objects.len() > 0 {
        map.insert(node, render_objects);
    }

    Ok(node)
}
fn get_absolute_positions(
    stretch: &mut Stretch,
    node: Node,
    parent_position: Point<f32>,
    map: &mut HashMap<Node, Vec<RenderObject>>,
    positioned_widgets: &mut Vec<PositionedRenderObject>,
) {
    let layout = stretch.layout(node).unwrap();
    let absolute_position = Point {
        x: parent_position.x + layout.location.x,
        y: parent_position.y + layout.location.y,
    };
    if map.contains_key(&node) {
        for render_object in map.remove(&node).unwrap() {
            positioned_widgets.push(PositionedRenderObject {
                render_object,
                position: absolute_position.clone(),
                size: layout.size,
                z_index: 0,
            })
        }
    }
    for child in stretch.children(node).unwrap() {
        get_absolute_positions(stretch, child, absolute_position, map, positioned_widgets);
    }
}


fn indent(text: String, indent_str: String) -> String {
    text.lines().into_iter().map(|line| format!("{}{}\n", indent_str, line)).collect()
}
pub fn print_layout(stretch: &Stretch, top_node: Node) -> String {
    let mut to_return = format!("{:?}\n", stretch.layout(top_node).unwrap());
    for child in stretch.children(top_node).unwrap() {
        to_return += indent(print_layout(&stretch, child), "    ".to_owned()).as_str();
    }
    to_return
}
