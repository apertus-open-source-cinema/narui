use crate::{heart::*, macros::widget, style::Size};
use lyon::{
    math::rect,
    path::{builder::*, Winding},
    tessellation::{
        path::{builder::BorderRadii, path::Builder},
        StrokeOptions,
    },
};
use std::sync::Arc;

#[widget(
    style = Default::default(),
    children = Default::default(),

    border_radius = 7.5,

    fill_color = Some(narui::theme::BG_LIGHT),
    stroke_color = None,
    stroke_options = Default::default()
)]
pub fn rounded_rect(
    style: Style,
    children: Vec<Fragment>,

    border_radius: f32,

    fill_color: Option<Color>,
    stroke_color: Option<Color>,
    stroke_options: StrokeOptions,

    context: Context,
) -> FragmentInner {
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

    let mut render_objects = vec![];
    if let Some(fill_color) = fill_color {
        render_objects.push((
            KeyPart::RenderObject(0),
            RenderObject::FillPath { path_gen: path_gen.clone(), color: fill_color },
        ));
    }
    if let Some(stroke_color) = stroke_color {
        render_objects.push((
            KeyPart::RenderObject(1),
            RenderObject::StrokePath {
                path_gen: path_gen.clone(),
                color: stroke_color,
                stroke_options,
            },
        ));
    }

    FragmentInner {
        children,
        layout_object: Some(LayoutObject { style, measure_function: None, render_objects }),
    }
}
