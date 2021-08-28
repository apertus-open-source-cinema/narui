/*
use crate::{style::*, *};
use palette::Shade;

#[widget(
    on_click = (| _context | {}),
    border_radius = Points(10.),
    color = crate::theme::BG,
    stroke_color = crate::theme::FG,
    style=Default::default(),
)]
pub fn button(
    on_click: impl Fn(Context) + Clone + Sync + Send + 'static,
    border_radius: BorderRadius,
    color: Color,
    stroke_color: Color,
    children: Vec<Fragment>,
    style: Style,
    context: Context,
) -> Fragment {
    let clicked = context.listenable(false);

    let color = if context.listen(clicked) {
        Color::from_linear(color.into_linear().lighten(0.1))
    } else {
        color
    };

    let callback = move |context: Context, is_clicked| {
        context.shout(clicked, is_clicked);
        if is_clicked {
            on_click(context);
        }
    };

    rsx! {
        <rect style=style fill_color=Some(color) stroke_color=Some(stroke_color) border_radius=border_radius>
            <input on_click=callback style={STYLE.padding(Points(10.))}>
                {children}
            </input>
        </rect>
    }
}

#[widget(min = 0.0, max = 1.0, width = 500.0, slide_color = crate::theme::BG_LIGHT, knob_color = crate::theme::FG)]
pub fn slider(
    val: f32,
    on_change: impl Fn(Context, f32) + Clone + Send + Sync + 'static,
    min: f32,
    max: f32,
    width: f32,
    slide_color: Color,
    knob_color: Color,
    context: Context,
) -> Fragment {
    let clicked = context.listenable(false);
    let on_click = move |context: Context, is_clicked| context.shout(clicked, is_clicked);
    let _click_start_val = context.listenable(val);

    let on_move = move |context: Context, position: Vec2| {
        let clicked = context.listen(clicked);
        /*
        let click_start_val = if clicked & clicked_changed {
            context.shout(click_start_val, position);
            position
        } else {
            context.listen(click_start_val)
        };
         */

        if clicked {
            //let position_delta = position - click_start_position;
            //let distance = (position.y - 10.).abs();
            //let distance_factor = if distance < 15.0 { 1. } else { 1. + distance / 50. };
            let new_val = (position.x / width * (max - min) + min).clamp(min, max);

            on_change(context, new_val);
        }
    };

    let slide_style = STYLE.width(Percent(1.0)).height(Points(5.0));
    let handle_container_style =
        STYLE.position_type(Absolute).top(Points(0.0)).left(Percent((val - min) / (max - min)));
    let handle_input_style = STYLE.left(Points(-10.));
    let handle_rect_style = STYLE.width(Points(20.)).height(Points(20.));
    let top_style = STYLE
        .width(Points(width))
        .height(Points(20.))
        .flex_direction(Column)
        .align_items(AlignItems::Stretch)
        .justify_content(JustifyContent::Center);
    rsx! {
         <input on_move=on_move style=top_style>
            <rect style=slide_style fill_color=Some(slide_color) />
            <container style=handle_container_style>
                <input on_click=on_click style=handle_input_style>
                    <rect border_radius=Percent(1.) style=handle_rect_style fill_color=Some(knob_color) />
                </input>
            </container>
         </input>
    }
}
 */
