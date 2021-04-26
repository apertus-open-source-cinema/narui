use std::ops::{Add, Sub};
use stretch::geometry::{Point, Size};
use winit::dpi::PhysicalPosition;

pub use palette::LinSrgba as Color;
use std::hash::{Hash, Hasher};
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
impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Self) -> Self::Output { Vec2 { x: self.x - rhs.x, y: self.y - rhs.y } }
}
impl Hash for Vec2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((self.x * 100.) as u64).hash(state);
        ((self.y * 100.) as u64).hash(state);
    }
}
impl Eq for Vec2 {} // TODO: this is evil and wrong but needed to be able to use as hashmap key
impl From<PhysicalPosition<f64>> for Vec2 {
    fn from(p: PhysicalPosition<f64>) -> Self { Vec2 { x: p.x as f32, y: p.y as f32 } }
}
impl<T: Into<f32>> From<Point<T>> for Vec2 {
    fn from(p: Point<T>) -> Self { Vec2 { x: p.x.into(), y: p.y.into() } }
}
impl<T: Into<f32>> From<Size<T>> for Vec2 {
    fn from(p: Size<T>) -> Self { Vec2 { x: p.width.into(), y: p.height.into() } }
}
impl Into<Size<f32>> for Vec2 {
    fn into(self) -> Size<f32> { Size { width: self.x, height: self.y } }
}
impl Into<Size<Number>> for Vec2 {
    fn into(self) -> Size<Number> {
        Size { width: Number::Defined(self.x), height: Number::Defined(self.y) }
    }
}
impl<T: Into<f32>> From<(T, T)> for Vec2 {
    fn from((x, y): (T, T)) -> Self { Vec2 { x: x.into(), y: y.into() } }
}
impl From<[u32; 2]> for Vec2 {
    fn from(arr: [u32; 2]) -> Self { Vec2 { x: arr[0] as f32, y: arr[1] as f32 } }
}
impl Into<[f32; 2]> for Vec2 {
    fn into(self) -> [f32; 2] { [self.x, self.y] }
}
impl Into<(f32, f32)> for Vec2 {
    fn into(self) -> (f32, f32) { (self.x, self.y) }
}
impl Into<lyon::math::Point> for Vec2 {
    fn into(self) -> lyon::math::Point { lyon::math::point(self.x, self.y) }
}
impl From<lyon::math::Point> for Vec2 {
    fn from(p: lyon::math::Point) -> Self { Vec2 { x: p.x, y: p.y } }
}


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
