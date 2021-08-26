// Qs for rutter_layout integration:
// - should it return a Vec of PositionedRenderObjects, or rather iter over
//   them, or do Key -> PositionedRenderObjects?
//

use crate::heart::*;
use hashbrown::HashMap;
use rutter_layout::{BoxConstraints, Layout, Offset};
use std::env;
use std::ops::Deref;

// PositionedRenderObject is the main output data structure of the Layouting
// pass It is like a regular RenderObject but with Positioning information added
#[derive(Debug, Clone)]
pub struct PositionedRenderObject<'a> {
    pub render_object: &'a RenderObject,
    pub rect: Rect,
    pub z_index: i32,
}


// A tree of layout Nodes that can be manipulated.
// This is the API with which the Evaluator commands the Layouter
pub trait LayoutTree {
    fn set_node(&mut self, key: &Key, layout: Box<dyn Layout>, render_object: Option<RenderObject>);
    fn remove_node(&mut self, key: &Key);
    fn set_children(&mut self, parent: &Key, children: &[Key]);
    fn get_positioned(&self, key: &Key) -> Option<(Rect, Option<&RenderObject>)>;
}

#[derive(Debug)]
struct BoxWithAdditional<T: ?Sized, A> {
    data: Box<T>,
    additional: A
}

impl<T: ?Sized, A> BoxWithAdditional<T, A> {
    fn new(data: Box<T>, additional: A) -> Self {
        Self {
            data,
            additional
        }
    }
}

impl<T: ?Sized, A> Deref for BoxWithAdditional<T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.data
    }
}

#[derive(Debug)]
pub struct Layouter {
    layouter: rutter_layout::Layouter<Key, BoxWithAdditional<dyn Layout, Option<RenderObject>>, ahash::RandomState>,
    debug_render_object: RenderObject,
    debug_layout_bounds: bool,
}

struct MaybeLayoutDebugIter<'a> {
    rect: Rect,
    item: Option<&'a RenderObject>,
    debug_render_object: &'a RenderObject,
    debug_layout_bounds: bool,
}

impl<'a> Iterator for MaybeLayoutDebugIter<'a> {
    type Item = PositionedRenderObject<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(render_object) = self.item.take() {
            Some(PositionedRenderObject { render_object, rect: self.rect, z_index: 0 })
        } else if self.debug_layout_bounds {
            self.debug_layout_bounds = false;
            Some(PositionedRenderObject {
                render_object: self.debug_render_object,
                rect: self.rect,
                z_index: 0,
            })
        } else {
            None
        }
    }
}

impl Layouter {
    pub fn new() -> Self {
        let debug_layout_bounds = env::var("NARUI_LAYOUT_BOUNDS").is_ok();

        Layouter {
            layouter: rutter_layout::Layouter::new(),
            debug_render_object: RenderObject::DebugRect,
            debug_layout_bounds,
        }
    }

    pub fn do_layout(&mut self, size: Vec2) {
        self.layouter.do_layout(
            BoxConstraints::tight_for(size.into()),
            Offset::zero(),
            Default::default(),
        );
    }

    #[cfg(debug_assertions)]
    pub fn iter_layouted(&self) -> impl Iterator<Item = PositionedRenderObject> {
        self.layouter.iter_with_obj(&Default::default()).flat_map(move |layout_item| MaybeLayoutDebugIter {
            rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
            item: layout_item.obj.additional.as_ref(),
            debug_layout_bounds: self.debug_layout_bounds,
            debug_render_object: &self.debug_render_object,
        })
    }

    #[cfg(not(debug_assertions))]
    pub fn iter_layouted(&self) -> impl Iterator<Item = PositionedRenderObject> {
        self.layouter.iter_with_obj(&Default::default()).filter_map(move |layout_item| {
            layout_item.obj.additional.as_ref().map(|render_object| {
                PositionedRenderObject {
                    rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                    z_index: 0,
                    render_object,
                }
            })
        })
    }
}

impl LayoutTree for Layouter {
    fn set_node(
        &mut self,
        key: &Key,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
    ) {
        self.layouter.set_node(&key, BoxWithAdditional::new(layout, render_object));
    }

    fn remove_node(&mut self, key: &Key) {
        self.layouter.remove(&key);
    }

    fn set_children<'a>(&mut self, parent: &Key, children: &[Key]) {
        self.layouter.set_children(parent, children.iter())
    }

    fn get_positioned(&self, key: &Key) -> Option<(Rect, Option<&RenderObject>)> {
        self.layouter.get_layout(key).map(|(offset, size, obj)| {
            (Rect { pos: offset.into(), size: size.into() }, obj.additional.as_ref())
        })
    }
}

fn indent(text: String, indent_str: String) -> String {
    text.lines().into_iter().map(|line| format!("{}{}\n", indent_str, line)).collect()
}
