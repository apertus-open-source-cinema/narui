use crate::heart::*;
use narui_derive::{hook, widget};
use stretch::style::Style;

#[widget(click = Default::default(), hover = Default::default(), position = Default::default(), style = Default::default())]
pub fn input(
    click: Option<StateValue<bool>>,
    hover: Option<StateValue<bool>>,
    position: Option<StateValue<Option<Vec2>>>,
    style: Style,
    children: Vec<Widget>,
) -> WidgetInner {
    let click = if let Some(v) = click { v } else { hook!(state(Default::default())) };
    let hover = if let Some(v) = hover { v } else { hook!(state(Default::default())) };
    let position = if let Some(v) = position { v } else { hook!(state(Default::default())) };

    WidgetInner::render_object(RenderObject::Input { hover, click, position }, children, style)
}
