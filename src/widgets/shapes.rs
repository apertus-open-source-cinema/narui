use crate::{
    api::{RenderObject, Widget},
    hooks::{state, Context},
    types::Color,
};
use narui_derive::{hook, rsx, widget};

use lyon::{
    math::rect,
    path::{builder::*, Winding},
    tessellation::path::{builder::BorderRadii, path::Builder},
};
use std::sync::Arc;
use stretch::{
    geometry::Size,
    node::MeasureFunc,
    style::{Dimension, Style},
};

#[widget(border_radius = 7.5, color = Color::grey())]
pub fn rounded_rect(border_radius: f32, color: Color, children: Vec<Widget>) -> Widget {
    let path_gen = Arc::new(move |size: Size<f32>| {
        let mut builder = Builder::new();
        builder.add_rounded_rectangle(
            &rect(0.0, 0.0, size.width, size.height),
            &BorderRadii {
                top_left: border_radius,
                top_right: border_radius,
                bottom_left: border_radius,
                bottom_right: border_radius,
            },
            Winding::Positive,
        );
        builder.build()
    });
    Widget::render_object(RenderObject::Path { path_gen, color }, children)
}
