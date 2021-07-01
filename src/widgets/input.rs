use crate::heart::*;
use narui_derive::{widget};
use stretch::style::Style;
use crate::hooks::*;
use std::sync::Arc;

#[widget(on_click = (|context, clicked| {}), on_hover = (|context, hovered| {}), on_move = (|context, position| {}), style = Default::default())]
pub fn input(
    on_click: impl Fn(Context, bool) -> () + Clone + Sync + Send + 'static,
    on_hover: impl Fn(Context, bool) -> () + Clone + Sync + Send + 'static,
    on_move: impl Fn(Context, Vec2) -> () + Clone + Sync + Send + 'static,
    style: Style,
    children: Widget,
    context: Context,
) -> Widget {
    Widget {
        children: children.into(),
        layout_object: Some(LayoutObject {
            style,
            measure_function: None,
            render_objects: vec![RenderObject::Input {
                on_click: Arc::new(on_click),
                on_hover: Arc::new(on_hover),
                on_move: Arc::new(on_move),
            }]
        })
    }
}
