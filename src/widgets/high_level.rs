use crate::{
    api::{RenderObject, Widget},
    hooks::{state, Context},
    types::Color,
    widgets::*,
};
use narui_derive::{hook, rsx, widget};
use stretch::style::Dimension;

#[widget(on_click = (|| {}))]
pub fn button(on_click: impl FnMut() -> (), children: Widget) -> Widget {
    rsx! {
        <padding all=Dimension::Points(10.)>
            <gesture_detector on_click={on_click} />
            <rounded_rect />
            <text>{"Hallo.".to_string()}</text>
        </padding>
    }
}


#[widget(initial_value = 0, step_size = 1)]
pub fn counter(initial_value: i32, step_size: i32) -> Widget {
    let count = hook!(state(initial_value));

    rsx! {
        <button on_click={|| count.set(*count + step_size)}>
            <text>{format!("{}", *count)}</text>
        </button>
    }
}
