use crate::*;
use narui::*;
use rutter_layout::{Maximal, Transparent};
use std::sync::Arc;

#[widget(on_click = (| _context, _clicked | {}), on_hover = (|_context, _hovered| {}), on_move = (|_context, _position| {}))]
pub fn input(
    on_click: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_hover: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_move: impl for<'a> Fn(&'a CallbackContext, Vec2) + Clone + 'static,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Leaf {
        layout: Box::new(Maximal),
        render_object: RenderObject::Input {
            on_click: Arc::new(on_click),
            on_hover: Arc::new(on_hover),
            on_move: Arc::new(on_move),
        },
    }
}

#[widget(on_click = (| _context, _clicked | {}), on_hover = (|_context, _hovered| {}), on_move = (|_context, _position| {}))]
pub fn input_composed(
    children: Vec<Fragment>,
    on_click: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_hover: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_move: impl for<'a> Fn(&'a CallbackContext, Vec2) + Clone + 'static,
    context: &mut WidgetContext,
) -> Fragment {
    rsx! {
        <stack size_using_first=true>
            <fragment>
                {children}
            </fragment>
            <input on_click = on_click on_hover = on_hover on_move = on_move />
        </stack>
    }
}
