use lyon::{
    lyon_tessellation::path::geom::Point,
    tessellation::{path::path::Builder, LineCap, StrokeOptions},
};
use narui::*;
use narui_widgets::*;
use palette::Shade;
use std::sync::Arc;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};
use rutter_layout::Maximal;
use lyon::lyon_tessellation::{FillTessellator, StrokeTessellator};
use narui::lyon_render::ColoredBuffersBuilder;


#[widget(on_drag = (|_context, _pos| {}), on_start = (|_context, _key| {}), on_end = (|_context, _key| {}))]
pub fn drag_detector(
    on_drag: impl Fn(&CallbackContext, Vec2) + Clone + Sync + Send + 'static,
    on_start: impl Fn(&CallbackContext, Key) + Clone + Sync + Send + 'static,
    on_end: impl Fn(&CallbackContext, Key) + Clone + Sync + Send + 'static,
    children: Vec<Fragment>,
    context: &mut WidgetContext,
) -> Fragment {
    let click_start_position = context.listenable(Vec2::zero());
    let click_started = context.listenable(false);
    let clicked = context.listenable(false);
    let key = context.widget_local.key;
    let on_click = move |context: &CallbackContext, clicked_current| {
        context.shout(clicked, clicked_current);
        if clicked_current {
            context.shout(click_started, true);
            on_start(context, key)
        } else {
            on_end(context, key)
        }
    };
    let on_move = move |context: &CallbackContext, position| {
        if context.spy(click_started) {
            context.shout(click_start_position, position);
            context.shout(click_started, false);
        } else if context.spy(clicked) {
            on_drag(context.clone(), position - (context.measure_size(key).unwrap() / 2.))
        }
    };

    rsx! {
        <stack size_using_first=true>
            <fragment>{children}</fragment>
            <input on_move=on_move on_click=on_click />
        </stack>
    }
}

#[widget]
pub fn hr(color: Color, context: &mut WidgetContext) -> Fragment {
    rsx! {
        <sized_box constraint={BoxConstraints::min_height(10.)}>
            <rect fill=Some(color) />
        </sized_box>
    }
}

#[widget(size = 20.0)]
pub fn handle(
    color: Color,
    size: f32,
    graph_root: Key,
    parent_node: Key,
    on_drag: impl Fn(&CallbackContext, Vec2, Vec2, Color) + Clone + Sync + Send + 'static,
    on_drag_end: impl Fn(&CallbackContext, Key, Key) + Clone + Sync + Send + 'static,
    on_drag_start: impl Fn(&CallbackContext, Key, Key) + Clone + Sync + Send + 'static,
    context: &mut WidgetContext,
) -> Fragment {
    let this_key = context.widget_local.key;
    let on_drag = move |context: &CallbackContext, pos: Vec2| {
        let size = context.measure_size(this_key).unwrap();
        let position = context.measure_offset(graph_root, this_key).unwrap();
        let start = position + (size / 2.);
        let end = start + pos;
        on_drag(context, start, end, color);
    };

    rsx! {
        <sized_box constraint={BoxConstraints::tight(size, size)}>
            <drag_detector
                on_drag=on_drag
                on_end=(move |context, key| {on_drag_end(context, key, parent_node)})
                on_start=(move |context, key| {on_drag_start(context, key, parent_node)})
            >
                <rect
                    fill=Some(color)
                    border_radius=Paxel(size)
                />
            </drag_detector>
        </sized_box>
    }
}

#[widget]
pub fn connection(start: Vec2, end: Vec2, color: Color, context: &mut WidgetContext) -> FragmentInner {
    let path_gen = Arc::new(move |size: Vec2, fill_tess: &mut FillTessellator, stroke_tess: &mut StrokeTessellator, mut buffers_builder: ColoredBuffersBuilder| {
        let mut builder = Builder::new();
        builder.begin(start.into());
        builder.cubic_bezier_to(
            Point::new((start.x + end.x) / 2.0, start.y),
            Point::new((start.x + end.x) / 2.0, end.y),
            end.into(),
        );
        builder.end(false);

        stroke_tess.tessellate_path(&builder.build(), &StrokeOptions::default().with_line_width(5.0), &mut buffers_builder.with_color(color));

    });
    let mut stroke_options = StrokeOptions::default();
    stroke_options.line_width = 5.;
    stroke_options.end_cap = LineCap::Round;
    stroke_options.start_cap = LineCap::Round;

    FragmentInner::Leaf { render_object: RenderObject::Path { path_gen }, layout: Box::new(Maximal) }
}

fn get_handle_offset(context: &CallbackContext, handle: Key, node: Key) -> Result<Vec2, MeasureError> {
    Ok(context.measure_offset(node, handle)? + (context.measure_size(handle)? / 2.))
}

#[widget]
pub fn node(
    name: impl ToString + Clone + Send + Sync + 'static,
    on_drag: impl Fn(&CallbackContext, Vec2) + Clone + Sync + Send + 'static,
    on_handle_drag: impl Fn(&CallbackContext, Vec2, Vec2, Color) + Clone + Sync + Send + 'static,
    on_handle_drag_start: impl Fn(&CallbackContext, Key, Key) + Clone + Sync + Send + 'static,
    on_handle_drag_end: impl Fn(&CallbackContext, Key, Key) + Clone + Sync + Send + 'static,
    graph_root: Key,
    context: &mut WidgetContext,
) -> Fragment {
    let key = context.widget_local.key;

    let fill_color = Color::from_linear(Shade::lighten(&BG_DARK.into_linear(), 0.1));
    let stroke_color = Color::from_linear(Shade::lighten(&BG_LIGHT.into_linear(), 0.2));

    rsx! {
        <sized_box constraint=BoxConstraints::tight(250., 150.)>
            <stack size_using_first=true>
                <column>
                    <drag_detector on_drag=on_drag>
                        <column main_axis_size=MainAxisSize::Min>
                            <text>{name}</text>
                            <hr color=stroke_color />
                        </column>
                    </drag_detector>

                    <flexible>
                        <stack>
                            <align alignment=Alignment::center_left()>
                                <column>
                                    <handle on_drag_end=on_handle_drag_end.clone() on_drag_start=on_handle_drag_start.clone() graph_root=graph_root parent_node=key on_drag=on_handle_drag.clone() color={color!(#ffff00)} />
                                    <handle on_drag_end=on_handle_drag_end.clone() on_drag_start=on_handle_drag_start.clone() graph_root=graph_root parent_node=key on_drag=on_handle_drag.clone() color={color!(#00ffff)} />
                                </column>
                            </align>
                            <align alignment=Alignment::center_right()>
                                <column>
                                    <handle on_drag_end=on_handle_drag_end.clone() on_drag_start=on_handle_drag_start.clone() graph_root=graph_root parent_node=key on_drag=on_handle_drag.clone() color={color!(#ff00ff)} />
                                </column>
                            </align>
                                /* TODO: add controls, etc */
                        </stack>
                    </flexible>
                </column>
                <rect border_radius=Paxel(10.0) fill=Some(fill_color) stroke=Some((stroke_color, 2.0)) />
            </stack>
        </sized_box>
    }
}

#[widget]
pub fn node_graph(context: &mut WidgetContext) -> Fragment {
    let this_key = context.widget_local.key;

    let nodes = context.listenable(vec![
        ("narui", Vec2::zero()),
        ("rocks", Vec2::new(300., 400.)),
        ("hard", Vec2::new(600., 400.)),
    ]);
    let current_nodes = context.listen(nodes);
    let current_nodes_clone = current_nodes.clone();

    let current_connection = context.listenable(None);
    let settled_connections: Listenable<Vec<((usize, Vec2), (usize, Vec2), Color)>> =
        context.listenable(vec![]);
    let on_handle_drag = move |context: &CallbackContext, start: Vec2, end: Vec2, color: Color| {
        context.shout(current_connection, Some((start, end, color)))
    };
    let drop_handle: Listenable<Option<(Key, Key, usize)>> = context.listenable(None);
    let drag_handle: Listenable<Option<(Key, Key, usize)>> = context.listenable(None);
    context.after_frame(move |context| {
        if context.spy(drop_handle).is_some() {
            let start = context.spy(drag_handle).unwrap();
            let end = context.spy(drop_handle).unwrap();
            let connection = (
                (start.2, get_handle_offset(context.clone(), start.0, start.1).unwrap()),
                (end.2, get_handle_offset(context.clone(), end.0, end.1).unwrap()),
                color!(#ffffff),
            );
            let mut connections = context.spy(settled_connections).clone();
            if let Some(i) = connections.iter().position(|x| x == &connection) {
                connections.remove(i);
            } else {
                connections.push(connection);
            }
            context.shout(settled_connections, connections);

            context.shout(drag_handle, None);
            context.shout(drop_handle, None);
        }
    });

    rsx! {
        <stack>
            <stack>
            {current_nodes.iter().cloned().enumerate().map(|(i, (name, position))| {
                let current_nodes_clone = current_nodes_clone.clone();

                rsx! {
                    <positioned pos=AbsolutePosition::from_offset(position.into()) key=i>
                         <node
                            name=name
                            graph_root=this_key
                            on_drag={move |context: &CallbackContext, pos: Vec2| {
                                let mut new_positions = current_nodes_clone.clone();
                                new_positions[i].1 = position + pos;
                                context.shout(nodes, new_positions);
                            }}
                            on_handle_drag=on_handle_drag.clone()
                            on_handle_drag_end={move |context: &CallbackContext, handle: Key, node: Key| {
                                if handle != context.spy(drag_handle).unwrap().0 {
                                    context.shout(drop_handle, Some((handle, node, i)));
                                }
                                context.shout(current_connection, None);
                            }}
                            on_handle_drag_start={move |context: &CallbackContext, handle, node| {
                                context.shout(drag_handle, Some((handle, node, i)));
                            }}
                        />
                    </positioned>
                }
            }).collect()}
        </stack>
        <fragment>
            {
                if let Some((start, end, color)) = context.listen(current_connection) {
                    vec![rsx! { <connection start=start end=end color=color /> }]
                } else { vec![] }
            }
        </fragment>
        <stack>
            {
                context.listen(settled_connections).iter().enumerate().map(|(i, (start, end, color))| {
                    let start = {
                        let (i, vec) = start;
                        context.listen(nodes)[*i].1 + *vec
                    };
                    let end = {
                        let (i, vec) = end;
                        context.listen(nodes)[*i].1 + *vec
                    };
                    rsx! {<connection key=i  start=start end=end color=*color />}
                }).collect()
            }
        </stack>
    </stack>
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
