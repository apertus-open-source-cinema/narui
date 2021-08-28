// Qs for rutter_layout integration:
// - should it return a Vec of PositionedRenderObjects, or rather iter over
//   them, or do Key -> PositionedRenderObjects?
//

use crate::heart::*;

use rutter_layout::{BoxConstraints, Idx, Layout, Offset};

use derivative::Derivative;
use std::ops::Deref;

// PositionedRenderObject is the main output data structure of the Layouting
// pass It is like a regular RenderObject but with Positioning information added
#[derive(Debug, Clone)]
pub struct PositionedRenderObject<'a> {
    pub render_object: &'a RenderObject,
    pub rect: Rect,
    pub z_index: u32,
}


// A tree of layout Nodes that can be manipulated.
// This is the API with which the Evaluator commands the Layouter
pub trait LayoutTree {
    fn add_node(&mut self, layout: Box<dyn Layout>, render_object: Option<RenderObject>) -> Idx;
    fn set_node(&mut self, key: Idx, layout: Box<dyn Layout>, render_object: Option<RenderObject>);
    fn remove_node(&mut self, key: Idx);
    fn set_children(&mut self, parent: Idx, children: &[Idx]);
    fn get_positioned(&self, key: Idx) -> (Rect, Option<&RenderObject>);
}

#[derive(Debug)]
struct BoxWithAdditional<T: ?Sized, A> {
    data: Box<T>,
    additional: A,
}

impl<T: ?Sized, A> BoxWithAdditional<T, A> {
    fn new(data: Box<T>, additional: A) -> Self { Self { data, additional } }
}

impl<T: ?Sized, A> Deref for BoxWithAdditional<T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target { &*self.data }
}

#[derive(Debug, Derivative)]
#[derivative(Default(new = "true"))]
pub struct Layouter {
    layouter: rutter_layout::Layouter<BoxWithAdditional<dyn Layout, Option<RenderObject>>>,
    #[derivative(Default(value = "RenderObject::DebugRect"))]
    debug_render_object: RenderObject,
}

impl Layouter {
    pub fn do_layout(&mut self, top: Idx, size: Vec2) {
        self.layouter.do_layout(BoxConstraints::tight_for(size.into()), Offset::zero(), top);
    }

    #[cfg(feature = "debug_bounds")]
    pub fn iter_layouted(&self, top: Idx) -> impl Iterator<Item = (Idx, PositionedRenderObject)> {
        let real = self.layouter.iter(top).filter_map(move |layout_item| {
            layout_item.obj.additional.as_ref().map(|render_object| {
                (
                    layout_item.idx,
                    PositionedRenderObject {
                        rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                        z_index: layout_item.z_index,
                        render_object,
                    },
                )
            })
        });
        let debug_rects = self.layouter.iter(Idx).map(|layout_item| {
            (
                Idx,
                PositionedRenderObject {
                    rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                    z_index: 0,
                    render_object: &RenderObject::DebugRect,
                },
            )
        });
        real.chain(debug_rects)
    }

    #[cfg(not(feature = "debug_bounds"))]
    pub fn iter_layouted(&self, top: Idx) -> impl Iterator<Item = (Idx, PositionedRenderObject)> {
        self.layouter.iter(top).filter_map(move |layout_item| {
            layout_item.obj.additional.as_ref().map(|render_object| {
                (
                    layout_item.idx,
                    PositionedRenderObject {
                        rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                        z_index: layout_item.z_index,
                        render_object,
                    },
                )
            })
        })
    }
}

impl LayoutTree for Layouter {
    fn add_node(&mut self, layout: Box<dyn Layout>, render_object: Option<RenderObject>) -> Idx {
        self.layouter.add_node(BoxWithAdditional::new(layout, render_object))
    }

    fn set_node(&mut self, idx: Idx, layout: Box<dyn Layout>, render_object: Option<RenderObject>) {
        self.layouter.set_node(idx, BoxWithAdditional::new(layout, render_object));
    }

    fn remove_node(&mut self, idx: Idx) { self.layouter.remove(idx); }

    fn set_children(&mut self, parent: Idx, children: &[Idx]) {
        self.layouter.set_children(parent, children.iter().cloned())
    }

    fn get_positioned(&self, idx: Idx) -> (Rect, Option<&RenderObject>) {
        let (offset, size, obj) = self.layouter.get_layout(idx);
        (Rect { pos: offset.into(), size: size.into() }, obj.additional.as_ref())
    }
}
