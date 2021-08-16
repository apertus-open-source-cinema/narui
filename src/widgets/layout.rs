use crate::{style::*, *};

fn layout_block(style: Style, children: Vec<Fragment>) -> FragmentInner {
    FragmentInner {
        children,
        layout_object: Some(LayoutObject { style, measure_function: None, render_objects: vec![] }),
    }
}

pub(crate) fn fill_parent_helper(style: Style, fill_parent: bool) -> Style {
    if fill_parent {
        style.width(Percent(1.0)).height(Percent(1.0))
    } else {
        style
    }
}

#[widget(style = Default::default())]
pub fn container(style: Style, children: Vec<Fragment>, context: Context) -> FragmentInner {
    layout_block(style, children)
}

#[widget(justify_content = Default::default(), align_items = Default::default(), style = Default::default(), fill_parent = true)]
pub fn column(
    justify_content: JustifyContent,
    align_items: AlignItems,
    fill_parent: bool,
    style: Style,
    children: Vec<Fragment>,
    context: Context,
) -> FragmentInner {
    let style = fill_parent_helper(style, fill_parent)
        .flex_direction(Column)
        .flex_wrap(NoWrap)
        .justify_content(justify_content)
        .align_items(align_items);
    layout_block(style, children)
}

#[widget(justify_content = Default::default(), align_items = Default::default(), fill_parent = true, style = Default::default())]
pub fn row(
    justify_content: JustifyContent,
    align_items: AlignItems,
    fill_parent: bool,
    style: Style,
    children: Vec<Fragment>,
    context: Context,
) -> FragmentInner {
    let style = fill_parent_helper(style, fill_parent)
        .flex_direction(Row)
        .flex_wrap(NoWrap)
        .justify_content(justify_content)
        .align_items(align_items);
    layout_block(style, children)
}

#[widget(width = Default::default(), height = Default::default(), style = Default::default(), children = Default::default())]
pub fn min_size(
    width: Dimension,
    height: Dimension,
    style: Style,
    children: Vec<Fragment>,
    context: Context,
) -> FragmentInner {
    let style = style.min_width(width).min_height(height);
    layout_block(style, children)
}
