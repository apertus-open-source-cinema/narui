use lyon::{
    math::rect as lyon_rect,
    path::{builder::*, Winding},
    tessellation::{
        path::{builder::BorderRadii, path::Builder},
        StrokeOptions,
        FillTessellator,
        StrokeTessellator,
    },
};
use narui::{heart::*, macros::widget};
use rutter_layout::Maximal;
use std::sync::Arc;
use narui::lyon_render::ColoredBuffersBuilder;
use lyon::algorithms::path::PathBuffer;


#[widget(border_radius = Default::default(), fill = None, stroke = None)]
pub fn rect(
    border_radius: Dimension,
    fill: Option<Color>,
    stroke: Option<(Color, f32)>,
    context: &mut WidgetContext,
) -> FragmentInner {
    let path_gen = Arc::new(move |size: Vec2, fill_tess: &mut FillTessellator, stroke_tess: &mut StrokeTessellator, mut buffers_builder: ColoredBuffersBuilder| {
        let border_radius_px = match border_radius {
            Paxel(px) => px,
            Fraction(percent) => {
                (if size.x > size.y { size.y } else { size.x }) * percent
            }
        };
        let border_radii = BorderRadii {
            top_left: border_radius_px,
            top_right: border_radius_px,
            bottom_left: border_radius_px,
            bottom_right: border_radius_px,
        };
        if let Some(fill) = fill {
            let mut builder = Builder::new();
            builder.add_rounded_rectangle(
                &lyon_rect(0.0, 0.0, size.x, size.y),
                &border_radii,
                Winding::Positive,
            );
            fill_tess.tessellate_path(&builder.build(), &Default::default(), &mut buffers_builder.with_color(fill));
        }
        if let Some((stroke, border_width)) = stroke {
            let mut builder = Builder::new();
            builder.add_rounded_rectangle(
                &lyon_rect(
                    border_width / 2.0,
                    border_width / 2.0,
                    size.x - border_width / 2.0,
                    size.y - border_width / 2.0,
                ),
                &border_radii,
                Winding::Positive,
            );
            stroke_tess.tessellate_path(&builder.build(), &Default::default(), &mut buffers_builder.with_color(stroke));
        }
    });

    FragmentInner::Leaf {
        render_object: RenderObject::Path { path_gen },
        layout: Box::new(Maximal),
    }
}
