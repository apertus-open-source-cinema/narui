// Qs for rutter_layout integration:
// - should it return a Vec of PositionedRenderObjects, or rather iter over
//   them, or do Key -> PositionedRenderObjects?
//

use crate::heart::*;

use rutter_layout::{BoxConstraints, Idx, Layout, LayoutIterDirection, Offset};

use derivative::Derivative;
use std::ops::Deref;

// PositionedRenderObject is the main output data structure of the Layouting
// pass It is like a regular RenderObject but with Positioning information added
#[derive(Debug, Clone)]
pub struct PositionedRenderObject<'a> {
    pub render_object: &'a RenderObject,
    pub clipping_rect: Option<Rect>,
    pub rect: Rect,
    pub z_index: u32,
}


// A tree of layout Nodes that can be manipulated.
// This is the API with which the Evaluator commands the Layouter
pub trait LayoutTree {
    fn add_node(
        &mut self,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
        is_clipper: bool,
    ) -> Idx;
    fn set_node(
        &mut self,
        key: Idx,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
        is_clipper: bool,
    );
    fn remove_node(&mut self, key: Idx);
    fn set_children(&mut self, parent: Idx, children: impl Iterator<Item = Idx>);
    fn get_positioned(&self, key: Idx) -> (Rect, Option<&RenderObject>);
}

#[derive(Debug)]
struct LayoutWithData {
    layout: Box<dyn Layout>,
    render_object: Option<RenderObject>,
    is_clipper: bool,
}
impl Deref for LayoutWithData {
    type Target = dyn Layout;
    fn deref(&self) -> &Self::Target { &*self.layout }
}

#[derive(Debug, Derivative)]
#[derivative(Default(new = "true"))]
pub struct Layouter {
    layouter: rutter_layout::Layouter<LayoutWithData>,
    #[derivative(Default(value = "RenderObject::DebugRect"))]
    debug_render_object: RenderObject,
}

impl Layouter {
    pub fn do_layout(&mut self, top: Idx, size: Vec2) {
        self.layouter.do_layout(BoxConstraints::tight_for(size.into()), Offset::zero(), top);
    }

    #[cfg(not(feature = "debug_bounds"))]
    pub fn iter_layouted(&self, top: Idx) -> impl Iterator<Item = (Idx, PositionedRenderObject)> {
        self.iter_layouted_internal(top)
    }

    fn iter_layouted_internal(
        &self,
        top: Idx,
    ) -> impl Iterator<Item = (Idx, PositionedRenderObject)> {
        use LayoutIterDirection::*;

        let mut parent_z_index = 0;
        let mut last_z_index_offset = 0;

        let mut clipper_stack = Vec::with_capacity(32);
        let mut last_clipper = None;

        self.layouter.iter(top).filter_map(move |(layout_item, direction_that_led_here)| {
            let current_rect = Rect { pos: layout_item.pos.into(), size: layout_item.size.into() };
            let z_index;
            match direction_that_led_here {
                Down => {
                    parent_z_index += last_z_index_offset;
                    z_index = parent_z_index;

                    if layout_item.obj.is_clipper {
                        if let Some(last_clipper) = last_clipper {
                            clipper_stack.push(last_clipper);
                        }
                        last_clipper = Some(current_rect);
                    }
                }
                Right => {
                    z_index = parent_z_index + layout_item.z_index_offset;
                    if layout_item.obj.is_clipper {
                        last_clipper = Some(current_rect);
                    }
                }
                Up => {
                    z_index = parent_z_index;
                    parent_z_index -= layout_item.z_index_offset;

                    if layout_item.obj.is_clipper {
                        last_clipper = None;
                        clipper_stack.pop();
                    }
                }
            }
            last_z_index_offset = layout_item.z_index_offset;

            layout_item.obj.render_object.as_ref().map(|render_object| {
                let positioned_render_object = PositionedRenderObject {
                    rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                    z_index,
                    render_object,
                    clipping_rect: last_clipper.or(clipper_stack.last().cloned()),
                };
                (layout_item.idx, positioned_render_object)
            })
        })
    }

    #[cfg(feature = "debug_bounds")]
    pub fn iter_layouted(&self, top: Idx) -> impl Iterator<Item = (Idx, PositionedRenderObject)> {
        let real = self.iter_layouted_internal(top);
        let debug_rects = self.layouter.iter(top).filter_map(|(layout_item, direction)| {
            if direction != LayoutIterDirection::Up {
                let positioned_render_object = PositionedRenderObject {
                    rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                    z_index: 0,
                    render_object: &RenderObject::DebugRect,
                    clipping_rect: None,
                };
                Some((layout_item.idx, positioned_render_object))
            } else {
                None
            }
        });
        real.chain(debug_rects)
    }
}

impl LayoutTree for Layouter {
    fn add_node(
        &mut self,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
        is_clipper: bool,
    ) -> Idx {
        self.layouter.add_node(LayoutWithData { layout, render_object, is_clipper })
    }

    fn set_node(
        &mut self,
        idx: Idx,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
        is_clipper: bool,
    ) {
        self.layouter.set_node(idx, LayoutWithData { layout, render_object, is_clipper });
    }

    fn remove_node(&mut self, idx: Idx) { self.layouter.remove(idx); }

    fn set_children(&mut self, parent: Idx, children: impl Iterator<Item = Idx>) {
        self.layouter.set_children(parent, children)
    }

    fn get_positioned(&self, idx: Idx) -> (Rect, Option<&RenderObject>) {
        let (offset, size, obj) = self.layouter.get_layout(idx);
        (Rect { pos: offset.into(), size: size.into() }, obj.render_object.as_ref())
    }
}
