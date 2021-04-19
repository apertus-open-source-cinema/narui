use crate::{
    api::{RenderObject, Widget},
    hooks::{state, Context},
};
use narui_derive::{hook, rsx, widget};

#[widget(on_click = (|| {}))]
pub fn gesture_detector(mut on_click: impl FnMut() -> ()) -> Widget {
    on_click();
    Widget::RenderObject(RenderObject::InputSurface)
}
