#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub fn rgb(r: f32, g: f32, b: f32) -> Self { Color { r, g, b, a: 1.0 }}
    pub fn white() -> Self { Color::rgb(1., 1., 1.) }
    pub fn grey() -> Self { Color::rgb(0.5, 0.5, 0.5) }
    pub fn black() -> Self { Color::rgb(0., 0., 0. ) }
    pub fn apertus_orange() -> Self { Color::rgb(0.98, 0.529, 0.337) }
}
impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        return [self.r, self.g, self.b, self.a]
    }
}