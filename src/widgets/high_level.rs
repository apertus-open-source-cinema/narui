use crate::{
    api::{RenderObject, Widget},
    hooks::{state, Context, StateValue},
    types::Color,
    widgets::*,
};
use narui_derive::{hook, rsx, widget};
use std::sync::Arc;
use stretch::style::Dimension;


#[widget(on_click = || {})]
pub fn button(on_click: impl FnMut() -> (), children: Widget) -> Widget {
    let is_hovered = hook!(state(false));
    let color = if *is_hovered { Color::apertus_orange() } else { Color::grey() };

    rsx! {
        <input hover=Some(is_hovered)>
            <rounded_rect color=color>
                <padding all=Dimension::Points(10.)>
                    {children}
                </padding>
            </rounded_rect>
        </input>
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
