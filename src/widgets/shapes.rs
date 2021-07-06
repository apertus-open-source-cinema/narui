use crate::{heart::*, macros::widget};

use crate::hooks::*;
use lyon::{
    math::rect,
    path::{builder::*, Winding},
    tessellation::{
        path::{builder::BorderRadii, path::Builder},
        StrokeOptions,
    },
};
use std::sync::Arc;
use stretch::{geometry::Size, style::Style};

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
    children: Fragment,

    border_radius: f32,

    fill_color: Option<Color>,
    stroke_color: Option<Color>,
    stroke_options: StrokeOptions,

    context: Context,
) -> Fragment {
    let path_gen = context.memoize_key(
        Key::default().with(KeyPart::Sideband {
            hash: KeyPart::calculate_hash(&("rounded_rect", (border_radius * 1000.0) as u64)),
        }),
        || {
            let closure: Arc<PathGenInner> = Arc::new(move |size: Size<f32>| {
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
            closure
        },
        (),
    );

    let mut render_objects = vec![];
    if let Some(fill_color) = fill_color {
        render_objects.push((
            KeyPart::RenderObject { number: 0 },
            RenderObject::FillPath { path_gen, color: fill_color },
        ));
    }
    if let Some(stroke_color) = stroke_color {
        render_objects.push((
            KeyPart::RenderObject { number: 1 },
            RenderObject::StrokePath { path_gen, color: stroke_color, stroke_options },
        ));
    }

    Fragment {
        key_part: context.widget_local.key.last_part(),
        children: children.into(),
        layout_object: Some(LayoutObject { style, measure_function: None, render_objects }),
    }
}
