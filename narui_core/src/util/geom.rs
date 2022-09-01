use crate::eval::layout::{Physical, ScaleFactor};
use glyph_brush::ab_glyph;
use std::ops::{Add, Div, Mul, Sub};


#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for crevice::std430::Vec2 {
    fn from(vec2: Vec2) -> crevice::std430::Vec2 { crevice::std430::Vec2 { x: vec2.x, y: vec2.y } }
}

impl Vec2 {
    pub fn zero() -> Self { Vec2 { x: 0.0, y: 0.0 } }
    pub fn new(x: f32, y: f32) -> Self { Vec2 { x, y } }
    pub fn min(&self, other: Vec2) -> Self {
        Vec2 { x: self.x.min(other.x), y: self.y.min(other.y) }
    }
    pub fn max(&self, other: Vec2) -> Self {
        Vec2 { x: self.x.max(other.x), y: self.y.max(other.y) }
    }
    pub fn with_x(&self, x: f32) -> Self { Self { x, ..*self } }
    pub fn with_y(&self, y: f32) -> Self { Self { y, ..*self } }
    pub fn maximum(&self) -> f32 { self.x.max(self.y) }
    pub fn pixels(&self) -> [u32; 2] { [self.x.round() as u32, self.y.round() as u32] }
}
impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Self) -> Self::Output { Vec2 { x: self.x + rhs.x, y: self.y + rhs.y } }
}
impl Add<f32> for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: f32) -> Self::Output { Vec2 { x: self.x + rhs, y: self.y + rhs } }
}
impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Self) -> Self::Output { Vec2 { x: self.x - rhs.x, y: self.y - rhs.y } }
}
impl Sub<f32> for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: f32) -> Self::Output { Vec2 { x: self.x - rhs, y: self.y - rhs } }
}
impl Div for Vec2 {
    type Output = Vec2;

    fn div(self, rhs: Self) -> Self::Output { Vec2 { x: self.x / rhs.x, y: self.y / rhs.y } }
}
impl Div<f32> for Vec2 {
    type Output = Vec2;

    fn div(self, rhs: f32) -> Self::Output { Vec2 { x: self.x / rhs, y: self.y / rhs } }
}
impl Mul for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: Self) -> Self::Output { Vec2 { x: self.x * rhs.x, y: self.y * rhs.y } }
}
impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: f32) -> Self::Output { Vec2 { x: self.x * rhs, y: self.y * rhs } }
}

macro_rules! implement_convert {
    ($typename:ty, $v:ident, $x:expr, $y:expr, $val:expr) => {
        impl From<$typename> for Vec2 {
            fn from($v: $typename) -> Self { Vec2 { x: $x, y: $y } }
        }
        impl From<Vec2> for $typename {
            fn from($v: Vec2) -> Self { $val }
        }
    };
}

macro_rules! impl_convert_tuple_arr2 {
    ($typename:ty) => {
        implement_convert!(
            ($typename, $typename),
            v,
            v.0 as f32,
            v.1 as f32,
            (v.x as $typename, v.y as $typename)
        );
        implement_convert!(
            [$typename; 2],
            v,
            v[0] as f32,
            v[1] as f32,
            [v.x as $typename, v.y as $typename]
        );
    };
}

impl_convert_tuple_arr2!(f32);
impl_convert_tuple_arr2!(f64);
impl_convert_tuple_arr2!(i32);
impl_convert_tuple_arr2!(i64);
impl_convert_tuple_arr2!(u32);
impl_convert_tuple_arr2!(u64);

implement_convert!(rutter_layout::Offset, v, v.x, v.y, rutter_layout::Offset { x: v.x, y: v.y });
implement_convert!(
    rutter_layout::Size,
    v,
    v.width,
    v.height,
    rutter_layout::Size { width: v.x, height: v.y }
);

implement_convert!(lyon::math::Point, v, v.x, v.y, lyon::math::point(v.x, v.y));
implement_convert!(ab_glyph::Point, v, v.x, v.y, ab_glyph::Point { x: v.x, y: v.y });


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rect {
    pub pos: Vec2,
    pub size: Vec2,
}
impl Rect {
    pub fn zero() -> Self { Self { pos: Vec2::zero(), size: Vec2::zero() } }
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.pos.x
            && point.y >= self.pos.y
            && point.x <= self.far_corner().x
            && point.y <= self.far_corner().y
    }
    pub fn from_corners(top_left: Vec2, bottom_right: Vec2) -> Self {
        let maybe_negative_size = bottom_right - top_left;
        Rect { pos: top_left, size: maybe_negative_size.max(Vec2::new(0.0, 0.0)) }
    }
    pub fn near_corner(&self) -> Vec2 { self.pos }
    pub fn far_corner(&self) -> Vec2 { self.pos + self.size }
    pub fn top_left_corner(&self) -> Vec2 {
        Vec2 {
            x: self.pos.x.min(self.pos.x + self.size.x),
            y: self.pos.y.min(self.pos.y + self.size.y),
        }
    }
    pub fn bottom_right_corner(&self) -> Vec2 {
        Vec2 {
            x: self.pos.x.max(self.pos.x + self.size.x),
            y: self.pos.y.max(self.pos.y + self.size.y),
        }
    }
    pub fn clip(&self, clipper: Rect) -> Rect {
        Rect::from_corners(
            Vec2::new(
                self.near_corner().x.max(clipper.near_corner().x),
                self.near_corner().y.max(clipper.near_corner().y),
            ),
            Vec2::new(
                self.far_corner().x.min(clipper.far_corner().x),
                self.far_corner().y.min(clipper.far_corner().y),
            ),
        )
    }
    pub fn center(&self) -> Vec2 { self.pos + self.size / 2.0 }
    pub fn inset(&self, val: f32) -> Self {
        Self { pos: self.pos + val, size: self.size - 2.0 * val }
    }
    pub fn minus_position(&self, pos: Vec2) -> Self { Self { pos: self.pos - pos, ..*self } }

    pub(crate) fn to_physical(self, scale_factor: ScaleFactor) -> Physical<Rect> {
        let Self { pos, size } = self;
        Physical::new(Rect { pos: pos * scale_factor.0, size: size * scale_factor.0 })
    }

    pub(crate) fn to_logical(self, scale_factor: ScaleFactor) -> Rect {
        let Self { pos, size } = self;
        Rect { pos: pos / scale_factor.0, size: size / scale_factor.0 }
    }
}
