use narui_core::{re_export::palette::rgb::Rgb, Color};
use std::marker::PhantomData;

const fn rgb(red: f32, green: f32, blue: f32) -> Color {
    Color { color: Rgb { red, green, blue, standard: PhantomData }, alpha: 1. }
}

pub static BG: Color = rgb(0.15686, 0.15686, 0.15686);
pub static BG_LIGHT: Color = rgb(0.31372, 0.31372, 0.31372);
pub static BG_DARK: Color = rgb(0., 0., 0.);

pub static FG: Color = rgb(1.0, 0.72156, 0.51764);
pub static FG_LIGHT: Color = rgb(0.75686, 0.34509, 0.16470);
pub static FG_DARK: Color = rgb(0.75686, 0.34509, 0.76078);

pub static TEXT_WHITE: Color = rgb(1., 1., 1.);
pub static TEXT_BLACK: Color = rgb(0., 0., 0.);
