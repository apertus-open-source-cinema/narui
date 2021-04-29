use crate::{heart::*, widgets::*};
use narui_derive::{hook, rsx, widget};
use palette::Shade;
use stretch::{
    geometry::Size,
    style::{AlignItems, Dimension, FlexDirection, JustifyContent, PositionType, Style},
};

#[widget]
pub fn frame_counter() -> Widget {
    let counter = hook!(state(0));
    counter.set(counter.get() + 1);
    rsx! {
        <text>{format!("{:.3}", counter.get())}</text>
    }
}


#[widget(click = None, color = crate::theme::BG)]
pub fn button(click: Option<StateValue<bool>>, color: Color, children: Widget) -> Widget {
    let click = if let Some(click) = click { click } else { hook!(state(false)) };

    let color = if click.get() { color.lighten(0.1) } else { color };
    rsx! {
        <input click=Some(click)>
            <rounded_rect color={color}>
                <padding all=Dimension::Points(10.)>
                    {children}
                </padding>
            </rounded_rect>
        </input>
    }
}

#[widget]
pub fn slider_demo() -> Widget {
    let value = hook!(state(24.0));

    let style = Style { align_items: AlignItems::FlexEnd, ..Default::default() };
    rsx! {
        <column fill_parent=false align_items=AlignItems::Center>
            <min_size height=Dimension::Points(300.0) style=style>
                <text size={value.get()}>{format!("{:.1} px", value.get())}</text>
            </min_size>
            <slider val=value min=12.0 max=300.0/>
        </column>
    }
}

#[widget(min = 0.0, max = 1.0, width = 500.0, slide_color = crate::theme::BG, knob_color = crate::theme::BG_LIGHT)]
pub fn slider(
    val: StateValue<f32>,
    min: f32,
    max: f32,
    width: f32,
    slide_color: Color,
    knob_color: Color,
) -> Widget {
    let last_value = hook!(state(val.get()));

    let position: StateValue<Option<Vec2>> = hook!(state(None));
    let last_position = hook!(state(Vec2::zero()));

    let click = hook!(state(false));
    let update_last = || {
        if let Some(position) = position.get() {
            last_position.set(position)
        }
        last_value.set(val.get());
    };
    hook!(rise_detector(click.clone(), update_last));

    if click.get() {
        if let Some(position) = position.get() {
            let position_delta = position - last_position.get();
            let distance = (position.y - 10.).abs();
            let distance_factor = if distance < 15.0 { 1. } else { 1. + distance / 50. };
            let new_val =
                position_delta.x / width * (max - min) / (distance_factor) + last_value.get();
            val.set(new_val.clamp(min, max));
            update_last();
        }
    }

    let slide_style = Style {
        size: Size { width: Dimension::Percent(1.0), height: Dimension::Points(5.0) },
        ..Default::default()
    };
    let handle_input_style = Style {
        position_type: PositionType::Absolute,
        position: stretch::geometry::Rect {
            top: Dimension::Points(0.0),
            start: Dimension::Percent((val.get() - min) / (max - min)),
            ..Default::default()
        },
        ..Default::default()
    };
    let handle_rect_style = Style {
        size: Size { width: Dimension::Points(20.0), height: Dimension::Points(20.0) },
        position: stretch::geometry::Rect { start: Dimension::Points(-10.), ..Default::default() },
        ..Default::default()
    };
    let top_style = Style {
        size: Size { width: Dimension::Points(width), height: Dimension::Points(20.) },
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Center,
        ..Default::default()
    };
    rsx! {
         <input position=position.clone().into() style=top_style>
            <rounded_rect style=slide_style color=slide_color>{vec![]}</rounded_rect>
            <input click=click.into() style=handle_input_style>
                <rounded_rect border_radius=10.0 style=handle_rect_style color=knob_color>{vec![]}</rounded_rect>
            </input>
         </input>
    }
}

#[widget(initial_value = 1, step_size = 1)]
pub fn counter(initial_value: i32, step_size: i32) -> Widget {
    let count = hook!(state(initial_value));

    rsx! {
         <row align_items=AlignItems::Center justify_content=JustifyContent::Center>
            <button click={hook!(on(|| count.set(count.get() - step_size)))}>
                <text>" - "</text>
            </button>
            <padding all=Dimension::Points(10.)>
                <text>{format!("{}", count.get())}</text>
            </padding>
            <button click={hook!(on(|| count.set(count.get() + step_size)))}>
                <text>" + "</text>
            </button>
         </row>
    }
}
