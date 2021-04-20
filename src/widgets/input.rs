use crate::heart::*;
use narui_derive::widget;
use std::sync::Arc;

#[widget(click = None, hover = None)]
pub fn input(
    click: Option<StateValue<bool>>,
    hover: Option<StateValue<bool>>,
    children: Vec<Widget>,
) -> Widget {
    Widget::render_object(RenderObject::Input { hover, click }, children)
}
