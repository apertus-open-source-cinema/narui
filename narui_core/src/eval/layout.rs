use crate::{
    geom::{Rect, Vec2},
    Fragment,
    RenderObject,
    SubPassSetup,
};
use derivative::Derivative;
use freelist::Idx;
use rutter_layout::{layouter::LayoutIterDirection, BoxConstraints, Layout, Offset};
use std::ops::Deref;
use winit::dpi::PhysicalPosition;

// type to create a seperation between logical and physical pixels
#[derive(Debug, Copy, Clone, Default)]
#[repr(transparent)]
pub struct Physical<T>(T);

impl<T> Physical<T> {
    pub fn new(v: T) -> Self { Self(v) }

    pub fn inner(&self) -> &T { &self.0 }

    pub fn map<R>(self, func: impl FnOnce(T) -> R) -> Physical<R> { Physical(func(self.0)) }

    // try to avoid using this function as much as possible
    pub fn unwrap_physical(self) -> T { self.0 }
}

pub trait ToPhysical {
    fn to_physical(self, scale_factor: ScaleFactor) -> Physical<Self>
    where
        Self: Sized;
}

impl ToPhysical for f32 {
    fn to_physical(self, scale_factor: ScaleFactor) -> Physical<Self> {
        Physical::new(self * scale_factor.0)
    }
}

impl ToPhysical for Vec2 {
    fn to_physical(self, scale_factor: ScaleFactor) -> Physical<Self> {
        Physical::new(self * scale_factor.0)
    }
}

impl Physical<Rect> {
    pub fn to_logical(self, scale_factor: ScaleFactor) -> Rect { self.0.to_logical(scale_factor) }
}

impl<T> Into<Physical<T>> for Physical<Physical<T>> {
    fn into(self) -> Physical<T> { self.unwrap_physical() }
}

impl Physical<Vec2> {
    pub fn to_logical(self, scale_factor: ScaleFactor) -> Vec2 { self.0 / scale_factor.0 }
}

impl From<PhysicalPosition<f64>> for Physical<Vec2> {
    fn from(v: PhysicalPosition<f64>) -> Physical<Vec2> {
        Physical::new((v.x as f32, v.y as f32).into())
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct ScaleFactor(pub f32);

/// PositionedElement is the main output data structure of the Layouting
/// pass. It attaches Position & other information to the RenderObject.
#[derive(Debug)]
pub struct PositionedElement<'a> {
    pub element: RenderObjectOrSubPass<'a>,
    pub clipping_rect: Option<Rect>,
    pub rect: Rect,
    pub z_index: u32,
}

/// PhysicalPositionedElement is the same as a PositionedElement but with
/// physical instead of logical coordinates.
#[derive(Debug)]
pub struct PhysicalPositionedElement<'a> {
    pub element: RenderObjectOrSubPass<'a>,
    pub clipping_rect: Option<Physical<Rect>>,
    pub rect: Physical<Rect>,
    pub z_index: u32,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum RenderObjectOrSubPass<'a> {
    RenderObject(&'a RenderObject),
    SubPassPush,
    SubPassPop(#[derivative(Debug = "ignore")] SubPassSetup),
}

/// A tree of layout Nodes that can be manipulated.
/// This is the API with which the Evaluator commands the Layouter
pub trait LayoutTree {
    fn add_node(
        &mut self,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
        is_clipper: bool,
        subpass: Option<SubPassSetup>,
        key: Fragment,
    ) -> Idx;
    fn set_node(
        &mut self,
        key: Idx,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
        is_clipper: bool,
        subpass: Option<SubPassSetup>,
        key: Fragment,
    );
    fn remove_node(&mut self, key: Idx);
    fn set_children(&mut self, parent: Idx, children: impl Iterator<Item = Idx>);

    // in logical / scaled pixels
    fn get_positioned_logical(&self, key: Idx) -> (Rect, Option<&RenderObject>);

    // in real / physical pixels
    fn get_positioned_physical(
        &self,
        key: Idx,
        scale_factor: ScaleFactor,
    ) -> (Physical<Rect>, Option<&RenderObject>);
}

#[derive(Derivative)]
#[derivative(Debug)]
struct LayoutWithData {
    layout: Box<dyn Layout>,
    render_object: Option<RenderObject>,
    is_clipper: bool,
    #[derivative(Debug = "ignore")]
    subpass: Option<SubPassSetup>,
    key: Fragment,
}
impl Deref for LayoutWithData {
    type Target = dyn Layout;
    fn deref(&self) -> &Self::Target { &*self.layout }
}

#[derive(Debug, Derivative)]
#[derivative(Default(new = "true"))]
pub struct Layouter {
    layouter: rutter_layout::layouter::Layouter<LayoutWithData>,
}

impl Layouter {
    pub fn do_layout(&mut self, top: Idx, size: Vec2) {
        self.layouter.do_layout(BoxConstraints::tight_for(size.into()), Offset::zero(), top);
    }

    pub fn iter_layouted_physical(
        &self,
        top: Idx,
        scale_factor: ScaleFactor,
    ) -> impl Iterator<Item = (Idx, PhysicalPositionedElement)> {
        self.iter_layouted(top).map(move |(idx, elem)| {
            (
                idx,
                PhysicalPositionedElement {
                    rect: elem.rect.to_physical(scale_factor),
                    clipping_rect: elem.clipping_rect.map(|r| r.to_physical(scale_factor)),
                    element: elem.element,
                    z_index: elem.z_index,
                },
            )
        })
    }

    #[cfg(not(feature = "debug_bounds"))]
    pub fn iter_layouted(&self, top: Idx) -> impl Iterator<Item = (Idx, PositionedElement)> {
        self.iter_layouted_internal(top)
    }

    fn iter_layouted_internal(&self, top: Idx) -> impl Iterator<Item = (Idx, PositionedElement)> {
        use LayoutIterDirection::*;

        let mut parent_z_index = 0;
        let mut last_z_index_offset = 0;

        let mut clipper_stack = Vec::with_capacity(32);
        let mut last_clipper = None;

        self.layouter.iter(top).filter_map(move |(layout_item, direction_that_led_here)| {
            let current_rect = Rect { pos: layout_item.pos.into(), size: layout_item.size.into() };
            let z_index;
            let preliminary_clipper = last_clipper.or_else(|| clipper_stack.last().cloned());
            let clip_fn = |rect: Rect| {
                if let Some(clipper) = preliminary_clipper {
                    rect.clip(clipper)
                } else {
                    rect
                }
            };
            match direction_that_led_here {
                Down => {
                    parent_z_index += last_z_index_offset;
                    z_index = parent_z_index;

                    if layout_item.obj.is_clipper {
                        if let Some(last_clipper) = last_clipper {
                            clipper_stack.push(last_clipper);
                        }
                        last_clipper = Some(clip_fn(current_rect));
                    }
                }
                Right => {
                    z_index = parent_z_index + layout_item.z_index_offset;
                    if layout_item.obj.is_clipper {
                        last_clipper = Some(clip_fn(current_rect));
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

            layout_item
                .obj
                .render_object
                .as_ref()
                .map(RenderObjectOrSubPass::RenderObject)
                .or_else(|| {
                    layout_item.obj.subpass.as_ref().map(|resolve| {
                        if matches!(direction_that_led_here, Right | Down) {
                            RenderObjectOrSubPass::SubPassPush
                        } else {
                            RenderObjectOrSubPass::SubPassPop(resolve.clone())
                        }
                    })
                })
                .map(|element| {
                    let positioned_render_object = PositionedElement {
                        rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                        z_index,
                        element,
                        clipping_rect: last_clipper.or_else(|| clipper_stack.last().cloned()),
                    };
                    (layout_item.idx, positioned_render_object)
                })
        })
    }

    #[cfg(feature = "debug_bounds")]
    pub fn iter_layouted(&self, top: Idx) -> impl Iterator<Item = (Idx, PositionedElement)> {
        let real = self.iter_layouted_internal(top);
        let debug_rects = self.layouter.iter(top).filter_map(|(layout_item, direction)| {
            if direction != LayoutIterDirection::Up {
                let positioned_render_object = PositionedElement {
                    rect: Rect { pos: layout_item.pos.into(), size: layout_item.size.into() },
                    z_index: 0,
                    element: RenderObjectOrSubPass::RenderObject(&RenderObject::DebugRect),
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
        subpass: Option<SubPassSetup>,
        key: Fragment,
    ) -> Idx {
        self.layouter.add_node(LayoutWithData { layout, render_object, is_clipper, subpass, key })
    }

    fn set_node(
        &mut self,
        idx: Idx,
        layout: Box<dyn Layout>,
        render_object: Option<RenderObject>,
        is_clipper: bool,
        subpass: Option<SubPassSetup>,
        key: Fragment,
    ) {
        self.layouter
            .set_node(idx, LayoutWithData { layout, render_object, is_clipper, subpass, key });
    }

    fn remove_node(&mut self, idx: Idx) { self.layouter.remove(idx); }

    fn set_children(&mut self, parent: Idx, children: impl Iterator<Item = Idx>) {
        self.layouter.set_children(parent, children)
    }

    fn get_positioned_logical(&self, idx: Idx) -> (Rect, Option<&RenderObject>) {
        let (offset, size, obj) = self.layouter.get_layout(idx);
        (Rect { pos: offset.into(), size: size.into() }, obj.render_object.as_ref())
    }

    fn get_positioned_physical(
        &self,
        idx: Idx,
        scale_factor: ScaleFactor,
    ) -> (Physical<Rect>, Option<&RenderObject>) {
        let (rect, obj) = self.get_positioned_logical(idx);
        (rect.to_physical(scale_factor), obj)
    }
}
