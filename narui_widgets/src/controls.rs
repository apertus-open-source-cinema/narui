use crate::*;
use narui_core::{re_export::palette::Shade, *};
use narui_macros::{rsx, widget};

#[widget]
pub fn button(
    #[default] on_click: impl for<'a> Fn(&'a CallbackContext) + Clone + 'static,
    #[default(Paxel(10.))] border_radius: Dimension,
    #[default(theme::BG)] color: Color,
    #[default(theme::FG)] stroke_color: Color,
    children: Fragment,
    context: &mut WidgetContext,
) -> Fragment {
    let clicked = context.listenable(false);

    let color = if context.listen(clicked) {
        Color::from_linear(Shade::lighten(&color.into_linear(), 0.1))
    } else {
        color
    };

    let callback = move |context: &CallbackContext, is_clicked, _, _| {
        context.shout(clicked, is_clicked);
        if is_clicked {
            on_click(context);
        }
    };

    rsx! {
        <stack fit=StackFit::Loose>
            <positioned>
                <rect_leaf fill=Some(color) stroke=Some((stroke_color, 1.0)) border_radius=border_radius />
            </positioned>
            <padding>
                <sized constraint=BoxConstraints::min_width(100.0)>
                    <align factor_width=Some(1.0) factor_height=Some(1.0)>{children}</align>
                </sized>
            </padding>
            <positioned>
                <input_leaf on_click=callback />
            </positioned>
        </stack>
    }
}

#[widget]
pub fn slider(
    val: f32,
    on_change: impl for<'a> Fn(&'a CallbackContext, f32) + Clone + 'static,
    #[default(0.0)] min: f32,
    #[default(1.0)] max: f32,
    #[default(theme::BG_LIGHT)] slide_color: Color,
    #[default(theme::FG)] knob_color: Color,
    context: &mut WidgetContext,
) -> Fragment {
    let widget_key = context.widget_local.idx;
    let clicked = context.listenable(false);
    let on_click =
        move |context: &CallbackContext, is_clicked, _, _| context.shout(clicked, is_clicked);

    let on_move = move |context: &CallbackContext, position: Vec2, _| {
        let clicked = context.spy(clicked);
        let width = context.measure_size(widget_key).unwrap().x - 20.0;

        if clicked {
            let new_val = ((position.x - 10.0) / width * (max - min) + min).clamp(min, max);
            on_change(context, new_val);
        }
    };

    rsx! {
        <sized constraint=BoxConstraints::default().with_tight_height(20.0)>
            <stack>
                <sized constraint=BoxConstraints::default().with_tight_height(5.0)>
                    <padding padding=EdgeInsets::horizontal(10.0)>
                        <rect_leaf border_radius=Fraction(1.) fill=Some(slide_color) /> // the slide
                    </padding>
                </sized>
                <input on_move = on_move>
                    <align alignment=Alignment::new(2.0 * (val - min) / (max - min) - 1.0, 0.0) factor_height = Some(1.0)>
                        <input on_click=on_click>
                            <rect border_radius=Fraction(1.) fill=Some(knob_color) constraint=BoxConstraints::fill().with_tight_width(20.0) />
                        </input>
                    </align>
                </input>
            </stack>
        </sized>
    }
}
