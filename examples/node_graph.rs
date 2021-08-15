use narui::{style::*, *};
use narui_macros::rsx_toplevel;
use palette::Shade;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};
use std::sync::Arc;
use lyon::tessellation::path::path::Builder;
use lyon::lyon_tessellation::path::geom::Point;
use lyon::tessellation::{StrokeOptions, LineCap};


#[widget(style = Default::default(), on_drag = (|_context, _pos| {}), on_start = (|_context| {}), on_end = (|_context| {}))]
pub fn drag_detector(
    style: Style,
    on_drag: impl Fn(Context, Vec2) + Clone + Sync + Send + 'static,
    on_start: impl Fn(Context) + Clone + Sync + Send + 'static,
    on_end: impl Fn(Context) + Clone + Sync + Send + 'static,
    children: Fragment,
    context: Context,
) -> Fragment {
    let click_start_position = context.listenable(Vec2::zero());
    let click_started = context.listenable(false);
    let clicked = context.listenable(false);
    let on_click = move |context: Context, clicked_current| {
        context.shout(clicked, clicked_current);
        if clicked_current {
            context.shout(click_started, true);
            on_start(context.clone())
        } else {
            on_end(context.clone())
        }
    };
    let on_move = move |context: Context, position| {
        if context.listen(click_started) {
            context.shout(click_start_position, position);
            context.shout(click_started, false);
        } else if context.listen(clicked) {
            on_drag(context.clone(), position - context.listen(click_start_position))
        }
    };

    rsx! {
        <input style=style on_move=on_move on_click=on_click>
            {children}
        </input>
    }
}

#[widget]
pub fn hr(color: Color, context: Context) -> Fragment {
    rsx! {
        <rounded_rect border_radius=0.0 fill_color=Some(color) >
            <min_size width={Dimension::Percent(1.0)} height={Dimension::Points(2.0)} />
        </rounded_rect>
    }
}

#[widget(size = 20.0)]
pub fn handle(
    color: Color,
    size: f32,
    graph_root: Key,
    on_drag: impl Fn(Context, Vec2, Vec2, Color) + Clone + Sync + Send + 'static,
    on_drag_end: impl Fn(Context) + Clone + Sync + Send + 'static,
    context: Context,
) -> Fragment {
    let this_key = context.widget_local.key;
    let on_drag = move |context: Context, pos: Vec2| {
        let size = context.measure_size(this_key).unwrap();
        let position = context.measure_offset(graph_root, this_key).unwrap();
        let start = position + (size / Vec2::new(2.0, 2.0));
        let end = start + pos;
        on_drag(context, start, end, color);
    };

    rsx! {
        <drag_detector on_drag=on_drag on_end=on_drag_end>
            <rounded_rect
                fill_color=Some(color)
                border_radius=size
                style={STYLE.width(Points(size)).height(Points(size))}
            />
        </drag_detector>
    }
}

#[widget]
pub fn connection(
    start: Vec2,
    end: Vec2,
    color: Color,
    context: Context,
) -> Fragment {
    dbg!(context.widget_local.key);
    let path_gen = Arc::new(move |size: Size<f32>| {
        let mut builder = Builder::new();
        builder.begin(Point::new(0., 0.));
        builder.cubic_bezier_to(
            Point::new(size.width / 2.0, 0.0),
            Point::new(size.width / 2.0, size.height),
            Point::new(size.width, size.height)
        );
        builder.end(false);
        builder.build()
    });
    let mut stroke_options = StrokeOptions::default();
    stroke_options.line_width = 5.;
    stroke_options.end_cap = LineCap::Round;
    stroke_options.start_cap = LineCap::Round;

    let render_objects = vec![
        (
            KeyPart::RenderObject(0),
            RenderObject::StrokePath {
                path_gen,
                color,
                stroke_options,
            },
        )
    ];

    let style = STYLE
        .position_type(Absolute)
        .top(Points(start.y))
        .left(Points(start.x))
        .width(Points(end.x - start.x))
        .height(Points(end.y - start.y));

    Fragment {
        key: context.widget_local.key,
        children: vec![],
        layout_object: Some(LayoutObject { style, measure_function: None, render_objects }),
    }
}


#[widget(style = Default::default())]
pub fn node(
    style: Style,
    on_drag: impl Fn(Context, Vec2) + Clone + Sync + Send + 'static,
    on_handle_drag: impl Fn(Context, Vec2, Vec2, Color) + Clone + Sync + Send + 'static,
    on_handle_drag_end: impl Fn(Context) + Clone + Sync + Send + 'static,
    graph_root: Key,
    context: Context,
) -> Fragment {
    let fill_color = Shade::lighten(&BG_DARK, 0.1);
    let stroke_color = Shade::lighten(&BG_LIGHT, 0.2);

    let handle_container_style = STYLE
        .position_type(Absolute)
        .height(Percent(1.0))
        .flex_direction(Column)
        .justify_content(SpaceEvenly);

    rsx! {
        <rounded_rect fill_color=Some(fill_color) stroke_color=Some(stroke_color) style={style.flex_direction(Column).align_items(AlignItems::Stretch)}>
            <drag_detector on_drag=on_drag style={STYLE.flex_direction(Column).flex_grow(1.0)} >
                <text style={STYLE.align_self(AlignSelf::Center)}>
                    {"GpuBitDepthConverter".to_string()}
                </text>
               <hr color=stroke_color />
            </drag_detector>

            <min_size width={Points(250.0)} height={Points(150.0)} >
                <container style={handle_container_style.left(Points(-10.))}>
                    <handle on_drag_end=on_handle_drag_end.clone() graph_root=graph_root on_drag=on_handle_drag.clone() color={color!(#ffff00)} />
                    <handle on_drag_end=on_handle_drag_end.clone() graph_root=graph_root on_drag=on_handle_drag.clone() color={color!(#00ffff)} />
                </container>
                <container style={handle_container_style.right(Points(-10.))}>
                    <handle on_drag_end=on_handle_drag_end.clone() graph_root=graph_root on_drag=on_handle_drag.clone() color={color!(#ff00ff)} />
                </container>

                /* TODO: add controls, etc in this area */
            </min_size>
        </rounded_rect>
    }
}


#[widget]
pub fn node_graph(context: Context) -> Fragment {
    let this_key = context.widget_local.key;

    let positions = context.listenable(vec![
        Vec2::new(300., 400.),
        Vec2::new(600., 400.),
        Vec2::new(500., 700.),
        Vec2::zero(),
    ]);
    let current_positions = context.listen(positions);
    let current_positions_clone = current_positions.clone();

    let connections = context.listenable(None);
    let on_handle_drag = move |context: Context, start: Vec2, end: Vec2, color: Color| {
        context.shout(connections, Some((start, end, color)))
    };
    let on_handle_drag_end = move |context: Context| {
        context.shout(connections, None);
    };
    rsx! {
        <container style=STYLE.width(Percent(1.0)).height(Percent(1.0))>
            <fragment>
            {current_positions.iter().cloned().enumerate().map(|(i, position)| {
                let current_positions_clone = current_positions_clone.clone();

                rsx! {
                     <node
                        key=&i
                        graph_root=this_key
                        style={STYLE.position_type(Absolute).top(Points(position.y)).left(Points(position.x))}
                        on_drag={move |context: Context, pos: Vec2| {
                            let mut new_positions = current_positions_clone.clone();
                            new_positions[i] = position + pos;
                            context.shout(positions, new_positions);
                        }}
                        on_handle_drag=on_handle_drag.clone()
                        on_handle_drag_end=on_handle_drag_end.clone()
                    />
                }
            }).collect_fragment(context.clone())}
        </fragment>
        <fragment>
            {
                if let Some((start, end, color)) = context.listen(connections) {
                    rsx! { <connection start=start end=end color=color /> }
                } else {
                    rsx! { <connection start=Vec2::zero() end=Vec2::new(100.0, 200.0) color=color!(#ffffff) /> }
                }
            }
        </fragment>
    </container>
    }
}


fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui node graph demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <node_graph />
        },
    );
}
