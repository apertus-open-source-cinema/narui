// a convenience helper to construct styles in a builder-pattern-esque fashion

use lazy_static::lazy_static;
use stretch::style::Style as StretchStyle;

pub use stretch::{
    prelude::{
        Dimension::{self, *},
        Number::{self, *},
        Rect,
        Size,
    },
    style::{
        AlignContent::{self, *},
        AlignItems::{self, *},
        AlignSelf::{self, *},
        Direction,
        Display::{self},
        FlexDirection::{self, *},
        FlexWrap::{self, *},
        JustifyContent::{self, *},
        Overflow::{self, *},
        PositionType::{self, *},
    },
};

lazy_static! {
    pub static ref STYLE: Style = Default::default();
}

macro_rules! normal_setter {
    ($name:ident, $type:ty) => {
        pub fn $name(self, $name: $type) -> Self { Self(StretchStyle { $name, ..self.0 }) }
    };
}

macro_rules! rect_setter {
    ($accessor:tt, $name:ident, $name_left:ident, $name_right:ident, $name_top:ident, $name_bottom:ident) => {
        pub fn $name(self, $name: Dimension) -> Self {
            Self(StretchStyle {
                $name: Rect { start: $name, end: $name, top: $name, bottom: $name },
                ..self.0
            })
        }
        pub fn $name_left(self, $name_left: Dimension) -> Self {
            Self(StretchStyle {
                $accessor: Rect { start: $name_left, ..self.0.$accessor },
                ..self.0
            })
        }
        pub fn $name_right(self, $name_right: Dimension) -> Self {
            Self(StretchStyle {
                $accessor: Rect { end: $name_right, ..self.0.$accessor },
                ..self.0
            })
        }
        pub fn $name_top(self, $name_top: Dimension) -> Self {
            Self(StretchStyle { $accessor: Rect { top: $name_top, ..self.0.$accessor }, ..self.0 })
        }
        pub fn $name_bottom(self, $name_bottom: Dimension) -> Self {
            Self(StretchStyle {
                $accessor: Rect { start: $name_bottom, ..self.0.$accessor },
                ..self.0
            })
        }
    };
}

macro_rules! size_setter {
    ($accessor:tt, $name_width:ident, $name_height:ident) => {
        pub fn $name_width(self, $name_width: Dimension) -> Self {
            Self(StretchStyle {
                $accessor: Size { width: $name_width, ..self.0.$accessor },
                ..self.0
            })
        }
        pub fn $name_height(self, $name_height: Dimension) -> Self {
            Self(StretchStyle {
                $accessor: Size { height: $name_height, ..self.0.$accessor },
                ..self.0
            })
        }
    };
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Style(StretchStyle);
impl Style {
    normal_setter!(display, Display);
    normal_setter!(position_type, PositionType);
    normal_setter!(direction, Direction);
    normal_setter!(flex_direction, FlexDirection);
    normal_setter!(flex_wrap, FlexWrap);
    normal_setter!(overflow, Overflow);
    normal_setter!(align_items, AlignItems);
    normal_setter!(align_self, AlignSelf);
    normal_setter!(align_content, AlignContent);
    normal_setter!(justify_content, JustifyContent);
    normal_setter!(flex_grow, f32);
    normal_setter!(flex_shrink, f32);
    normal_setter!(flex_basis, Dimension);
    normal_setter!(aspect_ratio, Number);

    rect_setter!(position, position, left, right, top, bottom);
    rect_setter!(margin, margin, margin_left, margin_right, margin_top, margin_bottom);
    rect_setter!(padding, padding, padding_left, padding_right, padding_top, padding_bottom);
    rect_setter!(border, border, border_left, border_right, border_top, border_bottom);

    size_setter!(size, width, height);
    size_setter!(min_size, min_width, min_height);
    size_setter!(max_size, max_width, max_height);

    pub(crate) fn stretch_style(&self) -> &StretchStyle { &self.0 }
}
