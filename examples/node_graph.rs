use narui::{style::*, *};
use narui_macros::rsx_toplevel;
use palette::Shade;
use winit::{platform::unix::WindowBuilderExtUnix, window::WindowBuilder};

#[widget]
pub fn hr(color: Color, context: Context) -> Fragment {
    rsx! {
        <rounded_rect border_radius=0.0 fill_color=Some(color) >
            <min_size width={Dimension::Percent(1.0)} height={Dimension::Points(2.0)} />
        </rounded_rect>
    }
}

#[widget(size = 20.0, on_drag = |_context, _pos| {} )]
pub fn handle(
    color: Color,
    size: f32,
    on_drag: impl Fn(Context, Vec2) + Clone + Sync + Send + 'static,
    context: Context,
) -> Fragment {
    rsx! {
        <drag_detector on_drag=on_drag>
            <rounded_rect
                fill_color=Some(color)
                border_radius=size
                style={STYLE.width(Points(size)).height(Points(size))}
            />
        </drag_detector>
    }
}

#[widget(style = Default::default())]
pub fn drag_detector(
    style: Style,
    on_drag: impl Fn(Context, Vec2) + Clone + Sync + Send + 'static,
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


#[widget(style = Default::default())]
pub fn node(
    style: Style,
    on_drag: impl Fn(Context, Vec2) + Clone + Sync + Send + 'static,
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
                    <handle color={color!(#ffff00)} />
                    <handle color={color!(#00ffff)} />
                </container>
                <container style={handle_container_style.right(Points(-10.))}>
                    <handle color={color!(#ff00ff)} />
                </container>

                /* TODO: add controls, etc in this area */
            </min_size>
        </rounded_rect>
    }
}


#[widget]
pub fn node_graph(context: Context) -> Fragment {
    // TODO: we need a way to measure a nodes (keys) size
    //  get the key of a child and get the position of a node relative to another
    // nodes position
    let positions = context.listenable(vec![
        Vec2::new(300., 400.),
        Vec2::new(600., 400.),
        Vec2::new(500., 700.),
        Vec2::zero(),
    ]);
    let current_positions = context.listen(positions);
    let current_positions_clone = current_positions.clone();
    rsx! {
        {current_positions.iter().cloned().enumerate().map(|(i, position)| {
            let current_positions_clone = current_positions_clone.clone();

            rsx! {
                 <node
                    key=&i
                    style={STYLE.position_type(Absolute).top(Points(position.y)).left(Points(position.x))}
                    on_drag={move |context: Context, pos: Vec2| {
                        let mut new_positions = current_positions_clone.clone();
                        new_positions[i] = position + pos;
                        context.shout(positions, new_positions);
                    }}
                />
            }
        }).collect_fragment(context.clone())}
    }
}


fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("narui slider demo")
        .with_gtk_theme_variant("dark".parse().unwrap());

    render(
        window_builder,
        rsx_toplevel! {
            <node_graph />
        },
    );
}
