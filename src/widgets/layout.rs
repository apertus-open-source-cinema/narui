use crate::{heart::*, hooks::*};
use crate::macros::widget;

use stretch::{
    geometry::{Rect, Size},
    style::{AlignItems, Dimension, FlexDirection, FlexWrap, JustifyContent, Style},
};

fn layout_block(style: Style, children: Widget, context: Context) -> Widget {
    Widget {
        key_part: context.widget_local.key.last_part(),
        children: children.into(),
        layout_object: Some(LayoutObject {
            style,
            measure_function: None,
            render_objects: vec![]
        })
    }
}

#[widget(style = Default::default())]
pub fn container(style: Style, children: Widget, context: Context) -> Widget {
    layout_block(style, children, context)
}

#[widget(justify_content = Default::default(), align_items = Default::default(), style = Default::default(), fill_parent = true)]
pub fn column(
    justify_content: JustifyContent,
    align_items: AlignItems,
    fill_parent: bool,
    style: Style,
    children: Widget,
    context: Context,
) -> Widget {
    let style = Style {
        flex_direction: FlexDirection::Column,
        flex_wrap: FlexWrap::NoWrap,
        size: Size {
            height: if fill_parent { Dimension::Percent(1.0) } else { Default::default() },
            width: if fill_parent { Dimension::Percent(1.0) } else { Default::default() },
        },
        justify_content,
        align_items,
        ..style
    };
    layout_block(style, children, context)
}

#[widget(justify_content = Default::default(), align_items = Default::default(), fill_parent = true, style = Default::default())]
pub fn row(
    justify_content: JustifyContent,
    align_items: AlignItems,
    fill_parent: bool,
    style: Style,
    children: Widget,
    context: Context,
) -> Widget {
    let style = Style {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::NoWrap,
        size: Size {
            height: if fill_parent { Dimension::Percent(1.0) } else { Default::default() },
            width: if fill_parent { Dimension::Percent(1.0) } else { Default::default() },
        },
        justify_content,
        align_items,
        ..style
    };
    layout_block(style, children, context)
}

#[widget(all=Default::default(), top_bottom=Default::default(), left_right=Default::default(), top=Default::default(), bottom=Default::default(), left=Default::default(), right=Default::default(), style = Default::default())]
pub fn padding(
    all: Dimension,
    top_bottom: Dimension,
    left_right: Dimension,
    top: Dimension,
    bottom: Dimension,
    left: Dimension,
    right: Dimension,
    style: Style,
    children: Widget,
    context: Context,
) -> Widget {
    let (mut t, mut b, mut l, mut r) = (all, all, all, all);
    if top_bottom != Dimension::default() {
        t = top_bottom;
        b = top_bottom;
    }
    if left_right != Dimension::default() {
        l = left_right;
        r = left_right;
    }
    if top != Dimension::default() {
        t = top
    }
    if bottom != Dimension::default() {
        b = bottom
    }
    if left != Dimension::default() {
        l = left
    }
    if right != Dimension::default() {
        r = right
    }

    let style = Style { padding: Rect { start: l, end: r, top: t, bottom: b }, ..style };
    layout_block(style, children, context)
}

#[widget(width = Default::default(), height = Default::default(), style = Default::default(), children = Default::default())]
pub fn min_size(
    width: Dimension,
    height: Dimension,
    style: Style,
    children: Widget,
    context: Context,
) -> Widget {
    let style = Style { min_size: Size { height, width }, ..style };
    layout_block(style, children, context)
}
