use crate::{
    AbsolutePosition,
    BoxConstraints,
    Layout,
    LayoutableChildren,
    Offset,
    Size,
    TraitComparable,
};
use derivative::Derivative;
use std::{
    any::Any,
    fmt::{Debug, Formatter},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Maximal;

impl Layout for Maximal {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        assert_eq!(children.len(), 0);
        (constraint.maximal_bounded(), 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transparent;

impl Layout for Transparent {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        if children.len() > 1 {
            panic!("Transparent Layout can have zero or one child but has {}", children.len())
        }
        if let Some(child) = children.into_iter().last() {
            let size = child.layout(constraint);
            child.set_pos(Offset::zero());
            child.set_z_index_offset(0);
            size
        } else {
            (constraint.constrain(Size::zero()), 1)
        }
    }

    fn query<'a>(
        &'a self,
        query: &dyn Any,
        children: LayoutableChildren<'a>,
    ) -> Option<&'a dyn Any> {
        assert!(children.len() <= 1);
        if let Some(child) = children.into_iter().last() {
            child.query(query)
        } else {
            None
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizedBox {
    constraint: BoxConstraints,
}

impl SizedBox {
    pub fn new(size: Size) -> Self { Self { constraint: BoxConstraints::tight_for(size) } }

    pub fn constrained(constraint: BoxConstraints) -> Self { Self { constraint } }
}

impl Layout for SizedBox {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        assert!(children.len() <= 1);
        if let Some(child) = children.into_iter().last() {
            let size = child.layout(self.constraint.enforce(constraint));
            child.set_pos(Offset::zero());
            child.set_z_index_offset(0);
            size
        } else {
            (self.constraint.enforce(constraint).constrain(Size::zero()), 1)
        }
    }
}

pub trait Positioner: Debug + PartialEq + 'static {
    fn position(&self, outer_size: Size, inner_size: Size) -> Offset;
}

pub trait Constrainer: Debug + PartialEq + 'static {
    fn constrain(&self, input_constraint: BoxConstraints) -> BoxConstraints;
}

pub trait Sizer: Debug + PartialEq + 'static {
    fn size(&self, input_constraint: BoxConstraints, child_size: Size) -> Size;
}

pub trait EmptySizer: Debug + PartialEq + 'static {
    fn size(&self, input_constraint: BoxConstraints) -> Size;
}

impl EmptySizer for Option<Size> {
    fn size(&self, input_constraint: BoxConstraints) -> Size {
        if let Some(empty_size) = self {
            *empty_size
        } else {
            input_constraint.maximal_bounded()
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SingleChildLayouter<P, C, S, E = Option<Size>> {
    positioner: P,
    constrainer: C,
    sizer: S,
    empty_sizer: E,
}

impl<P: Positioner, C: Constrainer, S: Sizer, E: EmptySizer> Layout
    for SingleChildLayouter<P, C, S, E>
{
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        assert!(children.len() <= 1);
        if let Some(child) = children.into_iter().last() {
            let (child_size, num_z_index) = child.layout(self.constrainer.constrain(constraint));
            let our_size = constraint.constrain(self.sizer.size(constraint, child_size));
            child.set_pos(self.positioner.position(our_size, child_size));
            child.set_z_index_offset(0);
            (our_size, num_z_index)
        } else {
            (constraint.constrain(self.empty_sizer.size(constraint)), 1)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoosenConstrainer;

impl Constrainer for LoosenConstrainer {
    fn constrain(&self, input_constraint: BoxConstraints) -> BoxConstraints {
        input_constraint.loosen()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedFractionalMaximalSizer {
    width_factor: Option<f32>,
    height_factor: Option<f32>,
}

impl BoundedFractionalMaximalSizer {
    fn new(width_factor: Option<f32>, height_factor: Option<f32>) -> Self {
        Self { width_factor, height_factor }
    }

    fn not_fractional() -> Self { Self { width_factor: None, height_factor: None } }
}

impl Sizer for BoundedFractionalMaximalSizer {
    fn size(&self, constraint: BoxConstraints, child_size: Size) -> Size {
        let target_size = child_size;

        let target_size = if !constraint.width_is_bounded() || self.width_factor.is_some() {
            target_size.scale_width(self.width_factor.unwrap_or(1.0))
        } else {
            target_size.maximize_width()
        };

        if !constraint.height_is_bounded() || self.height_factor.is_some() {
            target_size.scale_height(self.height_factor.unwrap_or(1.0))
        } else {
            target_size.maximize_height()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alignment {
    pub x: f32,
    pub y: f32,
}

impl Alignment {
    pub fn new(x: f32, y: f32) -> Self { Alignment { x, y } }
    pub fn top_left() -> Self { Alignment { x: -1.0, y: 1.0 } }
    pub fn top_center() -> Self { Alignment { x: 0.0, y: 1.0 } }
    pub fn top_right() -> Self { Alignment { x: 1.0, y: 1.0 } }
    pub fn center_left() -> Self { Alignment { x: -1.0, y: 0.0 } }
    pub fn center() -> Self { Alignment { x: 0.0, y: 0.0 } }
    pub fn center_right() -> Self { Alignment { x: 1.0, y: 0.0 } }
    pub fn bottom_left() -> Self { Alignment { x: -1.0, y: -1.0 } }
    pub fn bottom_center() -> Self { Alignment { x: 0.0, y: -1.0 } }
    pub fn bottom_right() -> Self { Alignment { x: 1.0, y: -1.0 } }
}

impl Positioner for Alignment {
    fn position(&self, outer_size: Size, inner_size: Size) -> Offset {
        let unit_x = (outer_size.width - inner_size.width) / 2.0;
        let unit_y = (outer_size.height - inner_size.height) / 2.0;
        Offset { x: unit_x + self.x * unit_x, y: unit_y - self.y * unit_y }
    }
}

pub type Align = SingleChildLayouter<Alignment, LoosenConstrainer, BoundedFractionalMaximalSizer>;

impl Align {
    pub fn new(alignment: Alignment) -> Self { Self::fractional(alignment, None, None) }

    pub fn fractional(
        alignment: Alignment,
        factor_width: Option<f32>,
        factor_height: Option<f32>,
    ) -> Self {
        SingleChildLayouter {
            positioner: alignment,
            constrainer: LoosenConstrainer,
            sizer: BoundedFractionalMaximalSizer::new(factor_width, factor_height),
            empty_sizer: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Paxel(f32),
    Fraction(f32),
}

impl Default for Dimension {
    fn default() -> Self { Self::Paxel(0.0) }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EdgeInsets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub fn all(val: f32) -> Self { Self { left: val, right: val, top: val, bottom: val } }

    pub fn horizontal(val: f32) -> Self { Self { left: val, right: val, top: 0.0, bottom: 0.0 } }

    pub fn vertical(val: f32) -> Self { Self { left: 0.0, right: 0.0, top: val, bottom: val } }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self { left: horizontal, right: horizontal, top: vertical, bottom: vertical }
    }

    pub fn specific(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self { left, right, top, bottom }
    }

    fn offset(self) -> Offset { Offset { x: self.left, y: self.top } }

    fn size(self) -> Size { Size { width: self.left + self.right, height: self.top + self.right } }
}

impl Constrainer for EdgeInsets {
    fn constrain(&self, input_constraint: BoxConstraints) -> BoxConstraints {
        input_constraint.deflate(*self)
    }
}

impl Sizer for EdgeInsets {
    fn size(&self, _input_constraint: BoxConstraints, child_size: Size) -> Size {
        child_size.inflate(*self)
    }
}

pub type Padding = SingleChildLayouter<AbsolutePosition, EdgeInsets, EdgeInsets>;

impl Padding {
    pub fn new(padding: EdgeInsets) -> Self {
        SingleChildLayouter {
            positioner: AbsolutePosition::from_offset(padding.offset()),
            constrainer: padding,
            sizer: padding,
            empty_sizer: Some(padding.size()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FractionalSize {
    x: Option<f32>,
    y: Option<f32>,
}

impl FractionalSize {
    fn constrain(&self, input_constraint: BoxConstraints) -> BoxConstraints {
        let constraint = if let Some(factor) = self.x {
            input_constraint.with_tight_width(input_constraint.max_width * factor)
        } else {
            input_constraint
        };

        if let Some(factor) = self.y {
            constraint.with_tight_height(constraint.max_height * factor)
        } else {
            constraint
        }
    }
}

impl Constrainer for FractionalSize {
    fn constrain(&self, input_constraint: BoxConstraints) -> BoxConstraints {
        self.constrain(input_constraint)
    }
}

impl EmptySizer for FractionalSize {
    fn size(&self, input_constraint: BoxConstraints) -> Size {
        self.constrain(input_constraint).constrain(Size::zero())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassthroughSizer;

impl Sizer for PassthroughSizer {
    fn size(&self, input_constraint: BoxConstraints, child_size: Size) -> Size {
        input_constraint.constrain(child_size)
    }
}

pub type FractionallySizedBox =
    SingleChildLayouter<Alignment, FractionalSize, PassthroughSizer, FractionalSize>;

impl FractionallySizedBox {
    pub fn new(size: FractionalSize) -> Self { Self::aligned(size, Alignment::center()) }

    pub fn aligned(size: FractionalSize, alignment: Alignment) -> Self {
        SingleChildLayouter {
            positioner: alignment,
            constrainer: size,
            sizer: PassthroughSizer,
            empty_sizer: size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AspectRatio {
    // width / height
    ratio: f32,
}

impl AspectRatio {
    fn width_for(self, height: f32) -> f32 { height * self.ratio }

    fn height_for(self, width: f32) -> f32 { width / self.ratio }

    fn target_size(self, input_constraint: BoxConstraints) -> Size {
        assert!(input_constraint.width_is_bounded() || input_constraint.height_is_bounded());

        let width = if input_constraint.width_is_bounded() {
            input_constraint.max_width
        } else {
            self.width_for(input_constraint.max_height).min(input_constraint.max_width)
        };
        let height = self.height_for(width).min(input_constraint.max_height);
        // TODO(robin): flutter does these, but I don't think these actually do
        // anything? let width =
        // self.width_for(height).max(input_constraint.min_width); let height =
        // self.height_for(width).max(input_constraint.min_height);
        let width = self.width_for(height);

        Size { width, height }
    }
}

impl Constrainer for AspectRatio {
    fn constrain(&self, input_constraint: BoxConstraints) -> BoxConstraints {
        let size = self.target_size(input_constraint);
        BoxConstraints::tight_for(input_constraint.constrain(size))
    }
}

impl EmptySizer for AspectRatio {
    fn size(&self, input_constraint: BoxConstraints) -> Size { self.target_size(input_constraint) }
}

pub type AspectRatioBox =
    SingleChildLayouter<AbsolutePosition, AspectRatio, PassthroughSizer, AspectRatio>;

impl AspectRatioBox {
    pub fn new(ratio: AspectRatio) -> Self {
        Self {
            positioner: AbsolutePosition::zero(),
            constrainer: ratio,
            sizer: PassthroughSizer,
            empty_sizer: ratio,
        }
    }
}

pub struct ClosureLayout {
    pub closure: Box<dyn Fn(BoxConstraints) -> Size>,
}
impl Debug for ClosureLayout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClosureLayout {{ ... }}")
    }
}

impl TraitComparable for ClosureLayout {
    fn as_any(&self) -> &dyn Any { self }

    fn as_trait_comparable(&self) -> &dyn TraitComparable { self }

    // TODO(robin): is there any meaningful way to compare Box<Closure>s?
    fn eq(&self, _other: &dyn TraitComparable) -> bool { false }
}

impl Layout for ClosureLayout {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        assert_eq!(children.len(), 0);
        ((self.closure)(constraint), 1)
    }
}
