// Qs for rutter_layout integration:
// - should it return a Vec of PositionedRenderObjects, or rather iter over them, or do Key -> PositionedRenderObjects?
//

use crate::{
    heart::*,
};
use hashbrown::HashMap;
use std::env;
use std::sync::Arc;
use rutter_layout::{BoxConstraints, Offset, Layout};


// PositionedRenderObject is the main output data structure of the Layouting
// pass It is like a regular RenderObject but with Positioning information added
#[derive(Debug, Clone)]
pub struct PositionedRenderObject<'a> {
    pub key: &'a Key,
    pub render_object: &'a RenderObject,
    pub rect: Rect,
    pub z_index: i32,
}


// A tree of layout Nodes that can be manipulated.
// This is the API with which the Evaluator commands the Layouter
pub trait LayoutTree {
    fn set_node(&mut self, key: &Key, layout: Arc<dyn Layout>, render_object: Option<RenderObject>);
    fn remove_node(&mut self, key: &Key);
    fn set_children(&mut self, parent: &Key, children: &[Key]);
    fn get_rect(&self, key: &Key) -> Option<Rect>;
}

pub struct Layouter {
    layouter: rutter_layout::Layouter<Key, Arc<dyn Layout>>,
    key_to_render_object: HashMap<Key, Option<RenderObject>>,
    debug_layout_bounds: bool,
}

impl Layouter {
    pub fn new() -> Self {
        let debug_layout_bounds = env::var("NARUI_LAYOUT_BOUNDS").is_ok();

        Layouter {
            layouter: rutter_layout::Layouter::new(),
            key_to_render_object: HashMap::new(),
            debug_layout_bounds
        }
    }

    pub fn do_layout(&mut self, size: Vec2) {
        self.layouter.do_layout(BoxConstraints::tight_for(size.into()), Offset::zero(), Default::default());
    }

    pub fn iter_layouted(&self) -> impl Iterator<Item=PositionedRenderObject> {
        self.layouter.iter(&Default::default()).filter_map(move |layout_item| {
            self.key_to_render_object[layout_item.key].as_ref().map(|render_object| {
                PositionedRenderObject {
                    key: layout_item.key,
                    render_object,
                    rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                    z_index: 0
                }
            })
        })
    }
}

impl LayoutTree for Layouter {
    fn set_node(&mut self, key: &Key, layout: Arc<dyn Layout>, render_object: Option<RenderObject>) {
        self.layouter.set_node(&key, layout);
        self.key_to_render_object.insert(key, render_object);
    }

    fn remove_node(&mut self, key: &Key) {
        self.layouter.remove(&key);
        self.key_to_render_object.remove(&key);
    }

    fn set_children<'a>(&mut self, parent: &Key, children: &[Key]) {
        self.layouter.set_children(parent, children)
    }

    fn get_rect(&self, key: &Key) -> Option<Rect> {
        self.layouter.get_layout(key).map(|(offset, size)| {
            Rect { pos: offset.into(), size: size.into() }
        })
    }
}


fn indent(text: String, indent_str: String) -> String {
    text.lines().into_iter().map(|line| format!("{}{}\n", indent_str, line)).collect()
}
