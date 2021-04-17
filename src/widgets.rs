use crate::{
    api::{RenderObject, TreeChildren, Widget},
    hooks::{state, Context},
};
use lyon::{
    math::{rect, Point},
    path::{builder::*, Winding},
    tessellation::path::{builder::BorderRadii, path::Builder},
};
use narui_derive::{hook, rsx, widget};
use stretch::{
    geometry::{Rect, Size},
    style::{AlignItems, Dimension, FlexDirection, FlexWrap, JustifyContent, PositionType, Style},
};

#[widget(width = Default::default(), height = Default::default())]
pub fn rounded_rect(width: Dimension, height: Dimension) -> Widget {
    let mut builder = Builder::new();
    builder.add_rounded_rectangle(
        &rect(0.0, 0.0, 100.0, 50.0),
        &BorderRadii { top_left: 10.0, top_right: 5.0, bottom_left: 20.0, bottom_right: 25.0 },
        Winding::Positive,
    );
    let path = builder.build();

    Widget {
        style: Style {
            size: Size { width, height },
            max_size: Size { width, height },
            ..Default::default()
        },
        children: TreeChildren::Leaf(RenderObject::Path(path)),
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
        children: TreeChildren::Children(
            children
                .into_iter()
                .map(|child| Widget {
                    style: child_style.clone(),
                    children: TreeChildren::Children(vec![child]),
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

    Widget { style, children: TreeChildren::Children(children) }
}


/*
#[widget(on_click = (|| {}))]
fn gesture_detector(mut on_click: impl FnMut() -> (), children: Vec<()>) -> Widget { on_click(); }


#[widget(size = 12.0, on_click = (|| {}))]
fn button(size: f32, mut on_click: impl FnMut() -> (), children: String) -> Widget {
    rsx! {
        <gesture_detector on_click={on_click}>
            <stacked>
                <rounded_rect/>
                <text size={size}>{children}</text>
            </stacked>
        </gesture_detector>
    }
}

#[widget(initial_value = 0, step_size = 1)]
fn counter(initial_value: i32, step_size: i32) -> Widget {
    let count = hook!(state(initial_value));

    rsx! {
        <button on_click={|| count.set(*count + step_size)}>
            {format!("{}", *count)}
        </button>
    }
}


#[widget]
fn text(size: f32, children: String) -> Widget {
    println!("{:#?}", children);
}

 */
