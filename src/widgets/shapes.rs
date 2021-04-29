use crate::heart::*;
use narui_derive::{hook, widget};

use lyon::{
    math::rect,
    path::{builder::*, Winding},
    tessellation::path::{builder::BorderRadii, path::Builder},
};
use std::sync::Arc;
use stretch::{geometry::Size, style::Style};

#[widget(border_radius = 7.5, color = crate::theme::BG_LIGHT, style = Default::default())]
pub fn rounded_rect(
    border_radius: f32,
    color: Color,
    style: Style,
    children: Vec<Widget>,
) -> WidgetInner<Widget> {
    let path_gen = hook!(effect_flat(
        || {
            let closure: PathGenInner = Arc::new(move |size: Size<f32>| {
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
        &format!("rounded_rect_cache_{}", border_radius)
    ));
    WidgetInner::render_object(RenderObject::Path { path_gen, color }, children, style)
}
