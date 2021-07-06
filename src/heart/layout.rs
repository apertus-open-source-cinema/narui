use crate::heart::*;
use hashbrown::HashMap;
use stretch::{
    node::{MeasureFunc, Node},
    prelude::{Dimension, Size, Style},
    Stretch,
};


// PositionedRenderObject is the main output data structure of the Layouting
// pass It is like a regular RenderObject but with Positioning information added
#[derive(Debug, Clone)]
pub struct PositionedRenderObject {
    pub key: Key,
    pub render_object: RenderObject,
    pub rect: Rect,
    pub z_index: i32,
}


// A tree of layout Nodes that can be manipulated.
// This is the API with which the Evaluator commands the Layouter
pub trait LayoutTree {
    fn set_node(&mut self, key: Key, layout_object: LayoutObject);
    fn remove_node(&mut self, key: Key);
    fn set_children(&mut self, parent: Key, children: impl Iterator<Item=Key>);
}

pub struct Layouter {
    stretch: Stretch,
    top_node: Option<Node>,

    key_node_map: HashMap<Key, Node>,
    render_object_map: HashMap<Node, Vec<(Key, RenderObject)>>,

    // this is a dirty hack because stretch does not support getting this information by itself.
    // this is a stretch deficiency
    node_has_measure: HashMap<Node, bool>,

    debug_layout_bounds: bool,
}

impl Layouter {
    pub fn new(debug_layout_bounds: bool) -> Self {
        let stretch = Stretch::new();
        let mut layouter = Layouter {
            stretch,
            top_node: None,
            debug_layout_bounds,
            key_node_map: HashMap::new(),
            render_object_map: HashMap::new(),
            node_has_measure: Default::default(),
        };
        layouter.top_node = Some(layouter.node(
            Default::default(),
            Style {
                size: Size { height: Dimension::Percent(1.0), width: Dimension::Percent(1.0) },
                ..Default::default()
            },
        ));
        layouter
    }
    pub fn do_layout(&mut self, size: Vec2) -> Result<Vec<PositionedRenderObject>, stretch::Error> {
        self.stretch.compute_layout(self.top_node.unwrap(), size.into())?;
        //println!("{}", self.layout_repr(self.top_node.unwrap()));

        let mut to_return = Vec::with_capacity(self.render_object_map.len());
        self.get_absolute_positions(self.top_node.unwrap(), Vec2::zero(), &mut to_return);
        Ok(to_return)
    }

    fn get_absolute_positions(
        &mut self,
        node: Node,
        parent_position: Vec2,
        positioned_widgets: &mut Vec<PositionedRenderObject>,
    ) {
        let layout = self.stretch.layout(node).unwrap();
        let pos = parent_position + layout.location.into();

        if self.debug_layout_bounds {
            positioned_widgets.push(PositionedRenderObject {
                // in principle this violates the key contract of uniqueness but it should not
                // matter here.
                key: Key::default().with(KeyPart::DebugLayoutBounds),
                render_object: RenderObject::DebugRect,
                rect: Rect { pos, size: layout.size.into() },
                z_index: 0,
            })
        }

        if self.render_object_map.contains_key(&node) {
            for (key, render_object) in self.render_object_map.get(&node).unwrap() {
                positioned_widgets.push(PositionedRenderObject {
                    key: *key,
                    render_object: render_object.clone(),
                    rect: Rect { pos, size: layout.size.into() },
                    z_index: 0,
                });
            }
        }
        for child in self.stretch.children(node).unwrap() {
            self.get_absolute_positions(child, pos, positioned_widgets);
        }
    }

    pub fn layout_repr(&self, node: Node) -> String {
        let (key, _) = self.key_node_map.iter().find(|(_, n)| **n == node).unwrap();
        let mut to_return = format!("{:?}\t{:?}\n", key, self.stretch.layout(node).unwrap());
        for child in self.stretch.children(node).unwrap() {
            to_return += indent(self.layout_repr(child), "    ".to_owned()).as_str();
        }
        to_return
    }

    pub fn node(&mut self, key: Key, style: Style) -> Node {
        if let Some(node) = self.key_node_map.get(&key) {
            return *node;
        }
        let new_node = self.stretch.new_node(style, &[]).unwrap();
        self.key_node_map.insert(key, new_node);
        self.node_has_measure.insert(new_node, false);
        new_node
    }
}

impl LayoutTree for Layouter {
    fn set_node(&mut self, key: Key, layout_object: LayoutObject) {
        let has_measure_function = layout_object.measure_function.is_some();
        let mut maybe_old_node = self.key_node_map.get(&key);
        // the maybe_old_node might be invalid, so we need to check if it is still
        // present in stretch
        if maybe_old_node.is_some() && self.stretch.style(*maybe_old_node.unwrap()).is_err()
        {
            maybe_old_node = None;
        }

        let node = match maybe_old_node {
            None => {
                match layout_object.measure_function {
                    Some(measure_function) => {
                        let measure_function = {
                            let measure_function = measure_function.clone();
                            MeasureFunc::Boxed(Box::new(move |size| measure_function(size)))
                        };
                        self.stretch
                            .new_leaf(layout_object.style, measure_function)
                            .unwrap()
                    }
                    None => {
                        self.stretch.new_node(layout_object.style, &[]).unwrap()
                    }
                }
            }
            Some(old_node) => {
                let old_node = *old_node;
                if self.stretch.style(old_node).unwrap() != &layout_object.style {
                    self.stretch.set_style(old_node, layout_object.style).unwrap();
                }
                match layout_object.measure_function {
                    Some(measure_function) => {
                        let measure_function = measure_function.clone();
                        let measure_function = MeasureFunc::Boxed(Box::new(move |size| {
                            measure_function(size)
                        }));
                        self.stretch.set_measure(old_node, Some(measure_function)).unwrap();
                        self.stretch.mark_dirty(old_node).unwrap();
                    }
                    None => {
                        if *self.node_has_measure.get(&old_node).unwrap() {
                            self.stretch.set_measure(old_node, None).unwrap();
                            self.stretch.mark_dirty(old_node).unwrap();
                        }
                    }
                }
                old_node
            }
        };
        self.node_has_measure.insert(node, has_measure_function);
        self.render_object_map.insert(
            node,
            layout_object
                .render_objects
                .into_iter()
                .map(|(key_part, render_object)| (key.with(key_part), render_object))
                .collect(),
        );
    }
    fn remove_node(&mut self, key: Key) {
        let node = self.key_node_map[&key];
        self.render_object_map.remove(&node).unwrap();
        self.node_has_measure.remove(&node).unwrap();
        self.stretch.remove(node);
    }
    fn set_children(&mut self, parent: Key, children: impl Iterator<Item=Key>) {
        let parent_node = self.node(parent, Default::default());
        let children: Vec<_> = children.map(|key| {
            self.node(key, Default::default()).clone()
        }).collect();
        self.stretch.set_children(parent_node, &children).unwrap();
    }

    /*fn set_children(&mut self, children: impl Iterator<Item = Key>, parent: Key) {
        let parent_node = match self.key_node_map.get(&parent) {
            Some(node) => *node,
            None => self.new_node(parent, Default::default()),
        };

        let old_children = self.stretch.children(parent_node).unwrap();
        let new_children: Vec<_> = children
            .map(|(key, layout_object)| {
                let has_measure_function = layout_object.measure_function.is_some();
                let mut maybe_old_node = self.key_node_map.get(&key);
                // the maybe_old_node might be invalid, so we need to check if it is still
                // present in stretch
                if maybe_old_node.is_some() && self.stretch.style(*maybe_old_node.unwrap()).is_err()
                {
                    maybe_old_node = None;
                }

                let node = match maybe_old_node {
                    None => {
                        match layout_object.measure_function {
                            Some(measure_function) => {
                                let measure_function = {
                                    let measure_function = measure_function.clone();
                                    MeasureFunc::Boxed(Box::new(move |size| measure_function(size)))
                                };
                                self.stretch
                                    .new_leaf(layout_object.style, measure_function)
                                    .unwrap()
                            }
                            None => {
                                self.stretch.new_node(layout_object.style, &[]).unwrap()
                                // we add the children later
                            }
                        }
                    }
                    Some(old_node) => {
                        let old_node = *old_node;
                        if self.stretch.style(old_node).unwrap() != &layout_object.style {
                            self.stretch.set_style(old_node, layout_object.style).unwrap();
                        }
                        match layout_object.measure_function {
                            Some(measure_function) => {
                                let measure_function = measure_function.clone();
                                let measure_function = MeasureFunc::Boxed(Box::new(move |size| {
                                    measure_function(size)
                                }));
                                self.stretch.set_measure(old_node, Some(measure_function)).unwrap();
                                self.stretch.mark_dirty(old_node).unwrap();
                            }
                            None => {
                                if *self.node_has_measure.get(&old_node).unwrap() {
                                    self.stretch.set_measure(old_node, None).unwrap();
                                    self.stretch.mark_dirty(old_node).unwrap();
                                }
                            }
                        }
                        old_node
                    }
                };
                self.node_has_measure.insert(node, has_measure_function);
                self.render_object_map.insert(
                    node,
                    layout_object
                        .render_objects
                        .into_iter()
                        .map(|(key_part, render_object)| (key.with(key_part), render_object))
                        .collect(),
                );
                node
            })
            .collect();

        if new_children != old_children {
            self.stretch.set_children(parent_node, new_children.as_slice()).unwrap();

            // now we need to clean up all nodes that are orphaned
            for node in old_children {
                if !new_children.contains(&node) {
                    self.render_object_map.remove(&node).unwrap();
                    self.node_has_measure.remove(&node).unwrap();
                    self.stretch.remove(node);
                }
            }
        }
    }

     */
}


fn indent(text: String, indent_str: String) -> String {
    text.lines().into_iter().map(|line| format!("{}{}\n", indent_str, line)).collect()
}
