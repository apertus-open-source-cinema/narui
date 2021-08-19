use crate::{
    heart::*,
    macros::widget,
    style::{Dimension, Size},
};
pub use lyon::tessellation::StrokeOptions;
use lyon::{
    math::rect,
    path::{builder::*, Winding},
    tessellation::path::{builder::BorderRadii, path::Builder},
};
use std::sync::Arc;

#[widget(
    style = Default::default(),
    children = Default::default(),

    border_radius = Default::default(),

    fill_color = None,
    stroke_color = None,
    stroke_options = Default::default()
)]
pub fn rounded_rect(
    style: Style,
    children: Vec<Fragment>,

    border_radius: Dimension,

    fill_color: Option<Color>,
    stroke_color: Option<Color>,
    stroke_options: StrokeOptions,

    context: Context,
) -> FragmentInner {
    let path_gen = Arc::new(move |size: Size<f32>| {
        let mut builder = Builder::new();
        let border_radius_px = match border_radius {
            Dimension::Undefined => 0.0,
            Dimension::Auto => 0.0,
            Dimension::Points(px) => px,
            Dimension::Percent(percent) => {
                (if size.width > size.height { size.height } else { size.width }) * percent
            }
        };
        builder.add_rounded_rectangle(
            &rect(0.0, 0.0, size.width, size.height),
            &BorderRadii {
                top_left: border_radius_px,
                top_right: border_radius_px,
                bottom_left: border_radius_px,
                bottom_right: border_radius_px,
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
