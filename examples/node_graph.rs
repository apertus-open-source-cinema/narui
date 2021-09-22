use narui::{
    layout::Maximal,
    re_export::{
        lyon::{
            lyon_tessellation::{path::geom::Point, FillTessellator, StrokeTessellator},
            tessellation::{path::path::Builder, LineCap, StrokeOptions},
        },
        palette::Shade,
    },
    renderer::ColoredBuffersBuilder,
    *,
};
use std::sync::Arc;


#[widget]
pub fn drag_detector(
    #[default] on_drag: impl Fn(&CallbackContext, Vec2) + Clone + Sync + Send + 'static,
    #[default] on_start: impl Fn(&CallbackContext, Fragment) + Clone + Sync + Send + 'static,
    #[default] on_end: impl Fn(&CallbackContext, Fragment) + Clone + Sync + Send + 'static,
    #[default] relative: bool,
    children: Fragment,
    context: &mut WidgetContext,
) -> Fragment {
    let click_start_position = context.listenable(Vec2::zero());
    let click_started = context.listenable(false);
    let clicked = context.listenable(false);
    let key = context.widget_local.idx;
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
            on_drag(
                context,
                position
                    - if relative {
                        context.spy(click_start_position)
                    } else {
                        context.measure_size(key).unwrap() / 2.
                    },
            )
        }
    };

    rsx! {
        <input on_move=on_move on_click=on_click>
            <fragment>{children.into()}</fragment>
        </input>
    }
}

#[widget]
pub fn hr(color: Color, context: &mut WidgetContext) -> Fragment {
    rsx! {
        <sized constraint={BoxConstraints::min_height(10.)}>
            <rect_leaf fill=Some(color) />
        </sized>
    }
}

#[widget]
pub fn handle(
    color: Color,
    #[default(20.0)] size: f32,
    graph_root: Fragment,
    parent_node: Fragment,
    on_drag: impl Fn(&CallbackContext, Vec2, Vec2, Color) + Clone + Sync + Send + 'static,
    on_drag_end: impl Fn(&CallbackContext, Fragment, Fragment) + Clone + Sync + Send + 'static,
    on_drag_start: impl Fn(&CallbackContext, Fragment, Fragment) + Clone + Sync + Send + 'static,
    context: &mut WidgetContext,
) -> Fragment {
    let this_key = context.widget_local.idx;
    let on_drag = move |context: &CallbackContext, pos: Vec2| {
        let size = context.measure_size(this_key).unwrap();
        let position = context.measure_offset(graph_root, this_key).unwrap();
        let start = position + (size / 2.);
        let end = start + pos;
        on_drag(context, start, end, color);
    };

    rsx! {
        <sized constraint={BoxConstraints::tight(size, size)}>
            <drag_detector
                on_drag=on_drag
                on_end=(move |context, key| {on_drag_end(context, key, parent_node)})
                on_start=(move |context, key| {on_drag_start(context, key, parent_node)})
            >
                <rect_leaf
                    fill=Some(color)
                    border_radius=Fraction(1.0)
                />
            </drag_detector>
        </sized>
    }
}

#[widget]
pub fn connection(
    start: Vec2,
    end: Vec2,
    color: Color,
    context: &mut WidgetContext,
) -> FragmentInner {
    let path_gen = Arc::new(
        move |_size: Vec2,
              _fill_tess: &mut FillTessellator,
              stroke_tess: &mut StrokeTessellator,
              mut buffers_builder: ColoredBuffersBuilder| {
            let mut builder = Builder::new();
            builder.begin(start.into());
            builder.cubic_bezier_to(
                Point::new((start.x + end.x) / 2.0, start.y),
                Point::new((start.x + end.x) / 2.0, end.y),
                end.into(),
            );
            builder.end(false);

            stroke_tess
                .tessellate_path(
                    &builder.build(),
                    &StrokeOptions::default().with_line_width(5.0),
                    &mut buffers_builder.with_color(color),
                )
                .unwrap();
        },
    );
    let mut stroke_options = StrokeOptions::default();
    stroke_options.line_width = 5.;
    stroke_options.end_cap = LineCap::Round;
    stroke_options.start_cap = LineCap::Round;

    FragmentInner::Leaf {
        render_object: RenderObject::Path { path_gen },
        layout: Box::new(Maximal),
    }
}

fn get_handle_offset(
    context: &CallbackContext,
    handle: Fragment,
    node: Fragment,
) -> Result<Vec2, MeasureError> {
    Ok(context.measure_offset(node, handle)? + (context.measure_size(handle)? / 2.))
}

#[widget]
pub fn node(
    name: impl ToString + Clone + Send + Sync + 'static,
    on_drag: impl Fn(&CallbackContext, Vec2) + Clone + Sync + Send + 'static,
    on_handle_drag: impl Fn(&CallbackContext, Vec2, Vec2, Color) + Clone + Sync + Send + 'static,
    on_handle_drag_start: impl Fn(&CallbackContext, Fragment, Fragment) + Clone + Sync + Send + 'static,
    on_handle_drag_end: impl Fn(&CallbackContext, Fragment, Fragment) + Clone + Sync + Send + 'static,
    graph_root: Fragment,
    context: &mut WidgetContext,
) -> Fragment {
    let key = context.widget_local.idx;

    let fill_color = Color::from_linear(Shade::lighten(&theme::BG_DARK.into_linear(), 0.1));
    let stroke_color = Color::from_linear(Shade::lighten(&theme::BG_LIGHT.into_linear(), 0.2));

    rsx! {
        <sized constraint=BoxConstraints::tight(250., 150.)>
            <stack>
                <padding padding=EdgeInsets::horizontal(10.0)>
                    <rect_leaf border_radius=Paxel(10.0) fill=Some(fill_color) stroke=Some((stroke_color, 2.0)) />
                </padding>
                <column>
                    <padding padding=EdgeInsets::horizontal(10.0)>
                        <drag_detector on_drag=on_drag relative=true>
                            <column main_axis_size=MainAxisSize::Min>
                                <text>{name}</text>
                                <hr color=stroke_color />
                            </column>
                        </drag_detector>
                    </padding>

                    <flexible fit=FlexFit::Tight>
                        <stack>
                            <align alignment=Alignment::center_left()>
                                <column main_axis_alignment=MainAxisAlignment::SpaceEvenly>
                                    <handle on_drag_end=on_handle_drag_end.clone() on_drag_start=on_handle_drag_start.clone() graph_root=graph_root parent_node=key on_drag=on_handle_drag.clone() color={Color::new(1., 1., 0., 1.)} />
                                    <handle on_drag_end=on_handle_drag_end.clone() on_drag_start=on_handle_drag_start.clone() graph_root=graph_root parent_node=key on_drag=on_handle_drag.clone() color={Color::new(0., 1., 1., 1.)} />
                                </column>
                            </align>
                            <align alignment=Alignment::center_right()>
                                <column>
                                    <handle on_drag_end=on_handle_drag_end on_drag_start=on_handle_drag_start graph_root=graph_root parent_node=key on_drag=on_handle_drag color={Color::new(1., 0., 1., 1.)} />
                                </column>
                            </align>
                                /* TODO: add controls, etc */
                        </stack>
                    </flexible>
                </column>
            </stack>
        </sized>
    }
}

#[widget]
pub fn node_graph(context: &mut WidgetContext) -> Fragment {
    let this_key = context.widget_local.idx;

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
    let drop_handle: Listenable<Option<(Fragment, Fragment, usize)>> = context.listenable(None);
    let drag_handle: Listenable<Option<(Fragment, Fragment, usize)>> = context.listenable(None);
    context.after_frame(move |context| {
        if context.spy(drop_handle).is_some() {
            let start = context.spy(drag_handle).unwrap();
            let end = context.spy(drop_handle).unwrap();
            let connection = (
                (start.2, get_handle_offset(context, start.0, start.1).unwrap()),
                (end.2, get_handle_offset(context, end.0, end.1).unwrap()),
                Color::new(1., 1., 1., 1.),
            );
            let mut connections = context.spy(settled_connections);
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
            <stack fit=StackFit::Tight>
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
                            on_handle_drag=on_handle_drag
                            on_handle_drag_end={move |context: &CallbackContext, handle: Fragment, node: Fragment| {
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
                    Some(rsx! { <connection start=start end=end color=color /> })
                } else {
                    None
                }
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
                    rsx! {<connection key=i start=start end=end color=*color />}
                }).collect()
            }
        </stack>
    </stack>
    }
}


fn main() {
    env_logger::init();
    app::render(
        app::WindowBuilder::new().with_title("narui node graph demo"),
        rsx_toplevel! {
            <node_graph />
        },
    );
}
