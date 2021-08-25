use crate::{heart::*, macros::widget};
pub use lyon::tessellation::StrokeOptions;
use lyon::{
    math::rect as lyon_rect,
    path::{builder::*, Winding},
    tessellation::path::{builder::BorderRadii, path::Builder},
};
use rutter_layout::Maximal;
use std::sync::Arc;

fn rounded_rect_builder(border_radius: Dimension, border_width: f32) -> Arc<PathGenInner> {
    Arc::new(move |size: Size| {
        let mut builder = Builder::with_capacity(4, 4);
        let border_radius_px = match border_radius {
            Paxel(px) => px,
            Fraction(percent) => {
                (if size.width > size.height { size.height } else { size.width }) * percent
            }
        };
        builder.add_rounded_rectangle(
            &lyon_rect(border_width / 2.0, border_width / 2.0, size.width - border_width / 2.0, size.height - border_width / 2.0),
            &BorderRadii {
                top_left: border_radius_px,
                top_right: border_radius_px,
                bottom_left: border_radius_px,
                bottom_right: border_radius_px,
            },
            Winding::Positive,
        );
        builder.build()
    })
}


#[widget(border_radius = Default::default(), color =  Default::default(), width = 1.0)]
pub fn border_rect(
    border_radius: Dimension,
    color: Color,
    width: f32,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Leaf {
        render_object: RenderObject::StrokePath {
            path_gen: rounded_rect_builder(border_radius, width),
            color,
            stroke_options: StrokeOptions::default().with_line_width(width),
        },
        layout: Box::new(Maximal),
    }
}

#[widget(border_radius = Default::default(), color =  Default::default())]
pub fn fill_rect(
    border_radius: Dimension,
    color: Color,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Leaf {
        render_object: RenderObject::FillPath {
            path_gen: rounded_rect_builder(border_radius, 0.0),
            color,
        },
        layout: Box::new(Maximal),
    }
}
