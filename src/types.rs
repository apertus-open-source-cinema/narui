use std::{cmp::Ordering, ops::Add};
use stretch::geometry::{Point, Size};
use winit::dpi::PhysicalPosition;

#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub fn rgb(r: f32, g: f32, b: f32) -> Self { Color { r, g, b, a: 1.0 } }
    pub fn white() -> Self { Color::rgb(1., 1., 1.) }
    pub fn grey() -> Self { Color::rgb(0.5, 0.5, 0.5) }
    pub fn black() -> Self { Color::rgb(0., 0., 0.) }
    pub fn apertus_orange() -> Self { Color::rgb(0.98, 0.529, 0.337) }
}
impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] { return [self.r, self.g, self.b, self.a] }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
impl Vec2 {
    pub fn zero() -> Self { Vec2 { x: 0.0, y: 0.0 } }
}
impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Self) -> Self::Output { Vec2 { x: self.x + rhs.x, y: self.y + rhs.y } }
}
impl From<PhysicalPosition<f64>> for Vec2 {
    fn from(p: PhysicalPosition<f64>) -> Self { Vec2 { x: p.x as f32, y: p.y as f32 } }
}
impl<T> From<Point<T>> for Vec2
where
    T: Into<f32>,
{
    fn from(p: Point<T>) -> Self { Vec2 { x: p.x.into(), y: p.y.into() } }
}
impl<T> From<Size<T>> for Vec2
where
    T: Into<f32>,
{
    fn from(p: Size<T>) -> Self { Vec2 { x: p.width.into(), y: p.height.into() } }
}
impl Into<Size<f32>> for Vec2 {
    fn into(self) -> Size<f32> { Size { width: self.x, height: self.y } }
}
impl<T> From<(T, T)> for Vec2
where
    T: Into<f32>,
{
    fn from((x, y): (T, T)) -> Self { Vec2 { x: x.into(), y: y.into() } }
}
impl Into<(f32, f32)> for Vec2 {
    fn into(self) -> (f32, f32) { (self.x, self.y) }
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
