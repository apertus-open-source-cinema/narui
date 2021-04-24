/* The layout pass takes the tree where the leaf nodes are Primitive Widgets and converts it to a list
of primitive PositionedPrimitiveWidget s that can then be handled to the rendering Backend.
 */

use crate::heart::*;
use std::collections::HashMap;
use stretch::{geometry::Size, node::Node, number::Number, Error, Stretch};


#[derive(Debug, Clone)]
pub struct PositionedRenderObject {
    pub render_object: RenderObject,
    pub rect: Rect,
    pub z_index: i32,
}

pub fn do_layout(
    top: Widget,
    size: Size<f32>,
) -> Result<Vec<PositionedRenderObject>, stretch::Error> {
    let mut stretch = stretch::node::Stretch::new();
    let mut map = HashMap::new();
    let top_node = add_widget_to_stretch(&top, &mut stretch, &mut map)?;

    stretch.compute_layout(
        top_node,
        Size { width: Number::Defined(size.width), height: Number::Defined(size.height) },
    )?;

    //println!("{}", print_layout(&stretch, top_node));

    let mut to_return = Vec::with_capacity(map.len());
    get_absolute_positions(&mut stretch, top_node, Vec2::zero(), &mut map, &mut to_return);

    Ok(to_return)
}
fn add_widget_to_stretch(
    widget: &Widget,
    stretch: &mut Stretch,
    map: &mut HashMap<Node, Vec<RenderObject>>,
) -> Result<Node, Error> {
    let (node, render_objects) = match &*widget.inner {
        WidgetInner::Composed { widget } => {
            let node = add_widget_to_stretch(widget, stretch, map)?;
            (node, vec![])
        }
        WidgetInner::Node { style, children, render_objects } => {
            let mut node_children = Vec::with_capacity(children.len());
            for child in children {
                node_children.push(add_widget_to_stretch(child, stretch, map)?);
            }
            let node = stretch.new_node(style.clone(), node_children)?;
            (node, render_objects.clone())
        }
        WidgetInner::Leaf { style, render_objects, .. } => {
            let widget = widget.clone();
            let measure_function = Box::new(move |size| match &*widget.inner {
                WidgetInner::Leaf { measure_function, .. } => measure_function(size),
                _ => unimplemented!(),
            });
            let node = stretch.new_leaf(style.clone(), measure_function)?;
            (node, render_objects.clone())
        }
    };

    if render_objects.len() > 0 {
        map.insert(node, render_objects);
    }

    Ok(node)
}
fn get_absolute_positions(
    stretch: &mut Stretch,
    node: Node,
    parent_position: Vec2,
    map: &mut HashMap<Node, Vec<RenderObject>>,
    positioned_widgets: &mut Vec<PositionedRenderObject>,
) {
    let layout = stretch.layout(node).unwrap();
    let pos = parent_position + layout.location.into();
    if map.contains_key(&node) {
        for render_object in map.remove(&node).unwrap() {
            positioned_widgets.push(PositionedRenderObject {
                render_object,
                rect: Rect { pos, size: layout.size.into() },
                z_index: 0,
            })
        }
    }
    for child in stretch.children(node).unwrap() {
        get_absolute_positions(stretch, child, pos, map, positioned_widgets);
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
