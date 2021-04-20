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


#[widget(on_click = || {}, color = crate::theme::BG)]
pub fn button(mut on_click: impl FnMut() -> (), color: Color, children: Widget) -> Widget {
    let is_clicked = hook!(state(false));
    let was_clicked = hook!(state(false));
    if is_clicked.get() && !was_clicked.get() {
        on_click()
    }
    was_clicked.set(is_clicked.get());

    let color = if is_clicked.get() { color.lighten(0.1) } else { color };
    rsx! {
        <input click=is_clicked.into()>
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
    let change = |v| {
        value.set(v);
    };
    rsx! {
        <column fill_parent=false>
            <text size={value.get()}>{format!("{:.3}", value.get())}</text>
            <slider val={value.get()} min=12.0 max=300.0 change={change}/>
        </column>
    }
}

#[widget(min = 0.0, max = 1.0, width = 500.0, slide_color = crate::theme::BG, knob_color = crate::theme::BG_LIGHT)]
pub fn slider(
    val: f32,
    min: f32,
    max: f32,
    width: f32,
    change: impl Fn(f32) -> (),
    slide_color: Color,
    knob_color: Color,
) -> Widget {
    let last_value = hook!(state(val));

    let position: StateValue<Option<Vec2>> = hook!(state(None));
    let last_position = hook!(state(Vec2::zero()));

    let click = hook!(state(false));
    let update_last = || {
        if let Some(position) = position.get() {
            last_position.set(position)
        }
        last_value.set(val);
    };
    hook!(rise_detector(click.clone(), update_last));

    if click.get() {
        if let Some(position) = position.get() {
            let position_delta = position - last_position.get();
            let distance = (position.y - 10.).abs();
            let distance_factor = if distance < 15.0 { 1. } else { 1. + distance / 50. };
            let val = position_delta.x / width * (max - min) / (distance_factor) + last_value.get();
            let val = if val < min {
                min
            } else if val > max {
                max
            } else {
                val
            };
            change(val);
            last_value.set(val);
            last_position.set(position);
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
            start: Dimension::Percent((val - min) / (max - min)),
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
            <button on_click={|| count.set(count.get() - step_size)}>
                <text>" - "</text>
            </button>
            <padding all=Dimension::Points(10.)>
                <text>{format!("{}", count.get())}</text>
            </padding>
            <button on_click={|| count.set(count.get() + step_size)}>
                <text>" + "</text>
            </button>
         </row>
    }
}
