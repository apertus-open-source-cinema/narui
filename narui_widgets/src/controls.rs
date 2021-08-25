use narui::*;
use crate::*;
use palette::Shade;

#[widget(
    on_click = (| _context | {}),
    border_radius = Paxel(10.),
    color = crate::theme::BG,
    stroke_color = crate::theme::FG,
)]
pub fn button(
    on_click: impl for<'a> Fn(&'a CallbackContext) + Clone + 'static,
    border_radius: Dimension,
    color: Color,
    stroke_color: Color,
    children: Vec<Fragment>,
    context: &mut WidgetContext,
) -> Fragment {
    let clicked = context.listenable(false);

    let color = if context.listen(clicked) {
        Color::from_linear(color.into_linear().lighten(0.1))
    } else {
        color
    };

    let callback = move |context: &CallbackContext, is_clicked| {
        context.shout(clicked, is_clicked);
        if is_clicked {
            on_click(context);
        }
    };

    rsx! {
        <stack fit=StackFit::Loose size_using_first=true>
            <padding><sized_box constraint=BoxConstraints::min_width(100.0)><align factor_width=Some(1.0) factor_height=Some(1.0)>{children}</align></sized_box></padding>
            <fill_rect color=color border_radius=border_radius />
            <border_rect color=stroke_color border_radius=border_radius />
            <input on_click=callback />
        </stack>
    }
}

#[widget(min = 0.0, max = 1.0, slide_color = crate::theme::BG_LIGHT, knob_color = crate::theme::FG)]
pub fn slider(
    val: f32,
    on_change: impl for<'a> Fn(&'a CallbackContext, f32) + Clone + 'static,
    min: f32,
    max: f32,
    slide_color: Color,
    knob_color: Color,
    context: &mut WidgetContext,
) -> Fragment {
    let widget_key = context.widget_local.key;
    let clicked = context.listenable(false);
    let on_click = move |context: &CallbackContext, is_clicked| context.shout(clicked, is_clicked);

    let on_move = move |context: &CallbackContext, position: Vec2| {
        let clicked = context.spy(clicked);
        let width = context.measure_size(widget_key).unwrap().x - 20.0;

        if clicked {
            let new_val = ((position.x - 10.0) / width * (max - min) + min).clamp(min, max);
            on_change(&context, new_val);
        }
    };

    rsx! {
        <sized_box constraint=BoxConstraints::default().with_tight_height(20.0)>
            <stack>
                <sized_box constraint=BoxConstraints::default().with_tight_height(5.0)>
                    <padding padding=EdgeInsets::horizontal(10.0)>
                        <fill_rect border_radius=Fraction(1.) color=slide_color /> // the slide
                    </padding>
                </sized_box>
                <input_composed on_move = on_move>
                    <align alignment=Alignment::new(2.0 * (val - min) / (max - min) - 1.0, 0.0) factor_height = Some(1.0)>
                        <input_composed on_click=on_click>
                            <sized_box constraint=BoxConstraints::default().with_tight_width(20.0)>
                                <fill_rect border_radius=Fraction(1.) color=knob_color />
                            </sized_box>
                        </input_composed>
                    </align>
                </input_composed>
            </stack>
        </sized_box>
    }
}
