use crate::{
    api::{RenderObject, Widget},
    hooks::{state, Context, StateValue},
};
use narui_derive::{hook, rsx, widget};
use std::sync::Arc;

#[widget(click = None, hover = None)]
pub fn input(
    click: Option<StateValue<bool>>,
    hover: Option<StateValue<bool>>,
    children: Vec<Widget>,
) -> Widget {
    Widget::render_object(RenderObject::Input { hover, click }, children)
}
