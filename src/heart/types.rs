use std::ops::{Add, Div, Mul, Sub};
use stretch::geometry::{Point, Size};
use winit::dpi::PhysicalPosition;

pub use palette::{rgb::Rgb as __Rgb, Srgba as Color};
use stretch::number::Number;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
impl Vec2 {
    pub fn zero() -> Self { Vec2 { x: 0.0, y: 0.0 } }
    pub fn new(x: f32, y: f32) -> Self { Vec2 { x, y } }
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

implement_convert!(Point<f64>, v, v.x as f32, v.y as f32, Point { x: v.x as f64, y: v.y as f64 });
implement_convert!(Point<f32>, v, v.x as f32, v.y as f32, Point { x: v.x as f32, y: v.y as f32 });
implement_convert!(
    Size<f64>,
    v,
    v.width as f32,
    v.height as f32,
    Size { width: v.x as f64, height: v.y as f64 }
);
implement_convert!(
    Size<f32>,
    v,
    v.width as f32,
    v.height as f32,
    Size { width: v.x as f32, height: v.y as f32 }
);
impl From<Vec2> for Size<Number> {
    fn from(v: Vec2) -> Self { Size { width: Number::Defined(v.x), height: Number::Defined(v.y) } }
}

implement_convert!(
    PhysicalPosition<f64>,
    v,
    v.x as f32,
    v.y as f32,
    PhysicalPosition { x: v.x as f64, y: v.y as f64 }
);

implement_convert!(lyon::math::Point, v, v.x, v.y, lyon::math::point(v.x, v.y));


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rect {
    pub pos: Vec2,
    pub size: Vec2,
}
impl Rect {
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.pos.x
            && point.y >= self.pos.y
            && point.x <= self.bottom_right().x
            && point.y <= self.bottom_right().y
    }
    pub fn bottom_right(&self) -> Vec2 { self.pos + self.size }
}
