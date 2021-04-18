use crate::{
    api::{Children, RenderObject, Widget},
    hooks::{state, Context},
};
use lyon::{
    math::rect,
    path::{builder::*, Winding},
    tessellation::path::{builder::BorderRadii, path::Builder},
};
use narui_derive::{hook, rsx, widget};
use std::sync::Arc;
use stretch::{
    geometry::{Point, Rect, Size},
    style::{AlignItems, Dimension, FlexDirection, FlexWrap, JustifyContent, PositionType, Style},
};

#[widget(width = Default::default(), height = Default::default(), border_radius = 10.0)]
pub fn rounded_rect(width: Dimension, height: Dimension, border_radius: f32) -> Widget {
    let path_gen = move |size: Size<f32>| {
        let mut builder = Builder::new();
        builder.add_rounded_rectangle(
            &rect(0.0, 0.0, size.width, size.height),
            &BorderRadii {
                top_left: border_radius,
                top_right: border_radius,
                bottom_left: border_radius,
                bottom_right: border_radius,
            },
            Winding::Positive,
        );
        builder.build()
    };

    Widget {
        style: Style {
            size: Size { width, height },
            max_size: Size { width, height },
            ..Default::default()
        },
        children: Children::RenderObject(RenderObject::Path(Arc::new(path_gen))),
    }
}

#[widget]
pub fn stacked(children: Vec<Widget>) -> Widget {
    let child_style = Style {
        size: Size { width: Dimension::Percent(1.0), height: Dimension::Percent(1.0) },
        position_type: PositionType::Absolute,
        position: Rect {
            start: Dimension::Points(0.0),
            top: Dimension::Points(0.0),
            ..Default::default()
        },
        ..Default::default()
    };

    let parent_style = Style {
        size: Size { width: Dimension::Percent(1.0), height: Dimension::Percent(1.0) },
        ..Default::default()
    };

    Widget {
        style: parent_style,
        children: Children::Composed(
            children
                .into_iter()
                .map(|child| Widget {
                    style: child_style.clone(),
                    children: Children::Composed(vec![child]),
                })
                .collect(),
        ),
    }
}

#[widget(justify_content = Default::default(), align_items = Default::default(), fill_parent = true)]
pub fn column(
    justify_content: JustifyContent,
    align_items: AlignItems,
    fill_parent: bool,
    children: Vec<Widget>,
) -> Widget {
    let style = Style {
        flex_direction: FlexDirection::Column,
        flex_wrap: FlexWrap::NoWrap,
        size: Size {
            height: if fill_parent { Dimension::Percent(1.0) } else { Default::default() },
            ..Default::default()
        },
        justify_content,
        align_items,
        ..Default::default()
    };

    Widget { style, children: Children::Composed(children) }
}


#[widget(on_click = (|| {}))]
fn gesture_detector(mut on_click: impl FnMut() -> ()) -> Widget {
    on_click();

    let style = Style {
        size: Size { height: Dimension::Percent(1.0), width: Dimension::Percent(1.0) },
        ..Default::default()
    };

    Widget { style, children: Children::RenderObject(RenderObject::InputSurface) }
}

#[widget(size = 12.0)]
fn text(size: f32, children: String) -> Widget {
    Widget {
        style: Style {
            size: Size {
                height: Dimension::Points(size),
                width: Dimension::Points(size * children.len() as f32),
            },
            ..Default::default()
        },
        children: Children::RenderObject(RenderObject::Text { text: children, size }),
    }
}

#[widget(on_click = (|| {}))]
fn button(on_click: impl FnMut() -> (), children: Widget) -> Widget {
    rsx! {
        <stacked>
            <gesture_detector on_click={on_click} />
            <rounded_rect/>
            {children}
        </stacked>
    }
}


#[widget(initial_value = 0, step_size = 1)]
fn counter(initial_value: i32, step_size: i32) -> Widget {
    let count = hook!(state(initial_value));

    rsx! {
        <button on_click={|| count.set(*count + step_size)}>
            <text>{format!("{}", *count)}</text>
        </button>
    }
}
