use crate::{heart::*, widgets::*};
use narui_derive::{hook, rsx, widget};
use palette::Shade;
use stretch::style::{AlignItems, Dimension, JustifyContent};


#[widget(on_click = || {}, color = crate::theme::BG_LIGHT)]
pub fn button(mut on_click: impl FnMut() -> (), color: Color, children: Widget) -> Widget {
    let is_clicked = hook!(state(false));
    let was_clicked = hook!(state(false));
    if *is_clicked && !*was_clicked {
        on_click()
    }
    was_clicked.set(*is_clicked);

    let color = if *is_clicked { color.lighten(0.1) } else { color };
    rsx! {
        <input click=Some(is_clicked)>
            <rounded_rect color={color}>
                <padding all=Dimension::Points(10.)>
                    {children}
                </padding>
            </rounded_rect>
        </input>
    }
}


#[widget(initial_value = 1, step_size = 1)]
pub fn counter(initial_value: i32, step_size: i32) -> Widget {
    let count = hook!(state(initial_value));

    rsx! {
         <row align_items=AlignItems::Center justify_content=JustifyContent::Center>
            <button on_click={|| count.set(*count - step_size)}>
                <text>" - "</text>
            </button>
            <padding all=Dimension::Points(10.)>
                <text>{format!("{}", *count)}</text>
            </padding>
            <button on_click={|| count.set(*count + step_size)}>
                <text>" + "</text>
            </button>
         </row>
    }
}
