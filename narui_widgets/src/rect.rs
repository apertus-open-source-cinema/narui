use crate::layout::{positioned, sized, stack};
use narui::{
    layout::Maximal,
    re_export::lyon::{
        algorithms::{
            math::{point, rect as lyon_rect},
            path::{
                builder::{BorderRadii, PathBuilder},
                Winding,
            },
        },
        lyon_algorithms::path::path::Builder,
        lyon_tessellation::{FillTessellator, StrokeOptions, StrokeTessellator},
    },
    renderer::ColoredBuffersBuilder,
    *,
};
use narui_macros::{rsx, widget};
use std::sync::Arc;


#[widget(border_radius = Dimension::default(), fill = None, stroke = None)]
pub fn rect_leaf(
    border_radius: Dimension,
    fill: Option<Color>,
    stroke: Option<(Color, f32)>,
    context: &mut WidgetContext,
) -> FragmentInner {
    let path_gen = Arc::new(
        move |size: Vec2,
              fill_tess: &mut FillTessellator,
              stroke_tess: &mut StrokeTessellator,
              mut buffers_builder: ColoredBuffersBuilder| {
            let border_radius_px = match border_radius {
                Paxel(px) => px,
                Fraction(percent) => (if size.x > size.y { size.y } else { size.x }) * percent,
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
                fill_tess
                    .tessellate_path(
                        &builder.build(),
                        &Default::default(),
                        &mut buffers_builder.with_color(fill),
                    )
                    .unwrap();
            }
            if let Some((stroke, border_width)) = stroke {
                let mut builder = Builder::new();
                builder.add_rounded_rectangle(
                    &lyon_rect(
                        border_width / 2.0,
                        border_width / 2.0,
                        size.x - border_width,
                        size.y - border_width,
                    ),
                    &border_radii,
                    Winding::Positive,
                );
                stroke_tess
                    .tessellate_path(
                        &builder.build(),
                        &StrokeOptions::default().with_line_width(border_width),
                        &mut buffers_builder.with_color(stroke),
                    )
                    .unwrap();
            }
        },
    );

    FragmentInner::Leaf {
        render_object: RenderObject::Path { path_gen },
        layout: Box::new(Maximal),
    }
}


#[widget(border_radius = Dimension::default(), fill = None)]
pub fn inverse_rect_leaf(
    border_radius: Dimension,
    fill: Option<Color>,
    context: &mut WidgetContext,
) -> FragmentInner {
    let path_gen = Arc::new(
        move |size: Vec2,
              fill_tess: &mut FillTessellator,
              _stroke_tess: &mut StrokeTessellator,
              mut buffers_builder: ColoredBuffersBuilder| {
            let br = match border_radius {
                Paxel(px) => px,
                Fraction(percent) => (if size.x > size.y { size.y } else { size.x }) * percent,
            };
            if let Some(fill) = fill {
                let mut builder = Builder::new();
                let m = 0.5522848;
                let mut corner = |a: Vec2, b: Vec2, c: Vec2| {
                    builder.begin(a.into());
                    builder.line_to(point(b.x, b.y));
                    builder.cubic_bezier_to(
                        point(b.x, (a.y - b.y) * m + b.y),
                        point((a.x - c.x) * m + c.x, c.y),
                        point(c.x, c.y),
                    );
                    builder.line_to(a.into());
                    builder.end(true);
                };
                corner(Vec2::zero(), Vec2::new(0., br / 2.), Vec2::new(br / 2., 0.));
                corner(
                    Vec2::new(0.0, size.y),
                    Vec2::new(0.0, size.y - br / 2.),
                    Vec2::new(br / 2., size.y),
                );
                corner(
                    Vec2::new(size.x, 0.0),
                    Vec2::new(size.x, br / 2.),
                    Vec2::new(size.x - br / 2., 0.0),
                );
                corner(size, size.with_y(size.y - br / 2.), size.with_x(size.x - br / 2.));

                fill_tess
                    .tessellate_path(
                        &builder.build(),
                        &Default::default(),
                        &mut buffers_builder.with_color(fill),
                    )
                    .unwrap();
            }
        },
    );

    FragmentInner::Leaf {
        render_object: RenderObject::Path { path_gen },
        layout: Box::new(Maximal),
    }
}

#[widget(
    border_radius = Default::default(),
    fill = None,
    stroke = None,
    do_clipping = false,
    constraint = Default::default(),
    children = None,
)]
pub fn rect(
    border_radius: Dimension,
    fill: Option<Color>,
    stroke: Option<(Color, f32)>,
    do_clipping: bool,
    constraint: narui::layout::BoxConstraints,
    children: Option<Fragment>,
    context: &mut WidgetContext,
) -> Fragment {
    if !do_clipping {
        if let Some(frag) = children {
            rsx! {
                <stack>
                    <positioned>
                        <rect_leaf border_radius=border_radius fill=fill stroke=stroke />
                    </positioned>
                    <sized constraint=constraint>{frag.into()}</sized>
                </stack>
            }
        } else {
            rsx! {
                <sized constraint=constraint><rect_leaf border_radius=border_radius fill=fill stroke=stroke /></sized>
            }
        }
    } else {
        rsx! {
            <stack is_clipper=true>
                <positioned>
                    <rect_leaf border_radius=border_radius fill=fill stroke=stroke />
                </positioned>
                <sized constraint=constraint>{children}</sized>
                <positioned>
                    <inverse_rect_leaf fill=Some(Color::new(0., 0., 0., 0.)) border_radius=border_radius />
                </positioned>
            </stack>
        }
    }
}
