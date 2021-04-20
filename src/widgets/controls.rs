use crate::{heart::*, widgets::*};
use narui_derive::{hook, rsx, widget};
use palette::Shade;
use stretch::{
    geometry::Size,
    style::{AlignItems, Dimension, FlexDirection, JustifyContent, PositionType, Style},
};


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
pub fn slider_example() -> Widget {
    let value = hook!(state(0.5));
    let change = |v| {
        value.set(v);
    };
    rsx! {
        <slider />
    }
}

#[widget]
pub fn slider() -> Widget {
    let value = hook!(state(0.5));
    let change = |v| {
        value.set(v);
    };

    let last_value = hook!(state(*value));
    let position: StateValue<Option<Vec2>> = hook!(state(None));
    let last_position = hook!(state(Vec2::zero()));

    let click = hook!(state(false));
    let update_last = || {
        last_position.set((*position).unwrap());
        last_value.set(*value);
    };
    hook!(rise_detector(click.clone(), update_last));

    if *click {
        if let Some(position) = *position {
            let position_delta = position - *last_position;
            let distance = 1.0 + ((position.y - 10.).abs() / 50.0);
            let val = position_delta.x / 300.0 / (distance) + *last_value;
            let val = if val < 0. {
                0.
            } else if val > 1. {
                1.
            } else {
                val
            };
            change(val);
            update_last()
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
            start: Dimension::Percent(*value),
            ..Default::default()
        },
        ..Default::default()
    };
    let handle_rect_style = Style {
        size: Size { width: Dimension::Points(20.0), height: Dimension::Points(20.0) },
        ..Default::default()
    };
    let top_style = Style {
        size: Size { width: Dimension::Points(300.), height: Dimension::Points(20.) },
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Center,
        ..Default::default()
    };
    rsx! {
         <input position=position.clone().into() style=top_style>
            <rounded_rect style=slide_style>{vec![]}</rounded_rect>
            <input click=click.into() style=handle_input_style>
                <rounded_rect border_radius=10.0 style=handle_rect_style>{vec![]}</rounded_rect>
            </input>
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
