/* The layout pass takes the tree where the leaf nodes are Primitive Widgets and converts it to a list
of primitive PositionedPrimitiveWidget s that can then be handled to the rendering Backend.
 */

use crate::heart::*;
use hashbrown::HashMap;
use stretch::{
    node::{MeasureFunc, Node},
    Error,
    Stretch,
};


#[derive(Debug, Clone)]
pub struct PositionedRenderObject {
    pub render_object: RenderObject,
    pub rect: Rect,
    pub z_index: i32,
}

pub struct Layouter {
    stretch: Stretch,
    key_node_map: HashMap<Key, Node>,
    last_map_size: usize,
}

impl Layouter {
    pub fn new() -> Self {
        Layouter { stretch: Stretch::new(), key_node_map: HashMap::new(), last_map_size: 0 }
    }
    pub fn do_layout(
        &mut self,
        top: Widget,
        size: Vec2,
    ) -> Result<Vec<PositionedRenderObject>, stretch::Error> {
        let mut map = HashMap::with_capacity(self.last_map_size);
        let top_node = self.add_widget_to_stretch(top.evaluated(), &mut map)?;
        self.last_map_size = map.len();
        self.stretch.compute_layout(top_node, size.into())?;

        // println!("{}", self.layout_repr(top_node));

        let mut to_return = Vec::with_capacity(map.len());
        self.get_absolute_positions(top_node, Vec2::zero(), &mut map, &mut to_return);
        Ok(to_return)
    }
    fn add_widget_to_stretch(
        &mut self,
        widget: &EvaluatedWidget,
        map: &mut HashMap<Node, Vec<RenderObject>>,
    ) -> Result<Node, Error> {
        let (node, render_objects) = match &*widget.inner {
            WidgetInner::Composed { widget } => {
                let node = self.add_widget_to_stretch(widget.lock().evaluated(), map)?;
                (node, vec![])
            }
            WidgetInner::Node { style, children, render_objects } => {
                let mut node_children = Vec::with_capacity(children.lock().len());
                for child in &*children.lock() {
                    node_children.push(self.add_widget_to_stretch(child.evaluated(), map)?);
                }
                match self.key_node_map.get(&widget.key) {
                    Some(node) => {
                        if widget.updated {
                            //println!("re layout: {:?}", &widget.key);
                            if self.stretch.style(node.clone())? != style {
                                //println!("different style: {:?}", style);
                                self.stretch.set_style(node.clone(), style.clone()).unwrap();
                            } else {
                                //println!("same style: {:?}", &style);
                            }
                            let prev_children = self.stretch.children(node.clone()).unwrap();
                            if prev_children != node_children {
                                for child in prev_children {
                                    self.stretch.remove_child(node.clone(), child)?;
                                }
                                self.stretch.set_children(node.clone(), &node_children).unwrap();
                            }
                        }
                        (node.clone(), render_objects.clone())
                    }
                    None => {
                        let node = self.stretch.new_node(style.clone(), &node_children)?;
                        self.key_node_map.insert(widget.key.clone(), node);
                        (node, render_objects.clone())
                    }
                }
            }
            WidgetInner::Leaf { style, render_objects, .. } => {
                let measure_function = || {
                    let closure_widget = widget.clone();
                    MeasureFunc::Boxed(Box::new(move |size| match &*closure_widget.inner {
                        WidgetInner::Leaf { measure_function, .. } => measure_function(size),
                        _ => unimplemented!(),
                    }))
                };


                let node = match self.key_node_map.get(&widget.key) {
                    Some(node) => {
                        if widget.updated {
                            //println!("re layout: {:?}", &widget.key);
                            self.stretch.set_measure(node.clone(), Some(measure_function()))?;
                        }
                        node.clone()
                    }
                    None => {
                        let node = self.stretch.new_leaf(style.clone(), measure_function())?;
                        self.key_node_map.insert(widget.key.clone(), node);
                        node
                    }
                };
                (node, render_objects.clone())
            }
        };

        if render_objects.len() > 0 {
            map.insert(node, render_objects);
        }

        Ok(node)
    }
    fn get_absolute_positions(
        &self,
        node: Node,
        parent_position: Vec2,
        map: &mut HashMap<Node, Vec<RenderObject>>,
        positioned_widgets: &mut Vec<PositionedRenderObject>,
    ) {
        let layout = self.stretch.layout(node).unwrap();
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
        for child in self.stretch.children(node).unwrap() {
            self.get_absolute_positions(child, pos, map, positioned_widgets);
        }
    }
    pub fn layout_repr(&self, node: Node) -> String {
        let mut to_return = format!("{:?}\n", self.stretch.layout(node).unwrap());
        for child in self.stretch.children(node).unwrap() {
            to_return += indent(self.layout_repr(child), "    ".to_owned()).as_str();
        }
        to_return
    }
}

fn indent(text: String, indent_str: String) -> String {
    text.lines().into_iter().map(|line| format!("{}{}\n", indent_str, line)).collect()
}
