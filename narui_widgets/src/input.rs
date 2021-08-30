use narui::*;
use rutter_layout::Maximal;
use std::sync::Arc;

use crate::{fragment, positioned, stack};

#[widget(on_click = (| _context, _clicked | {}), on_hover = (|_context, _hovered| {}), on_move = (|_context, _position| {}))]
pub fn input_leaf(
    on_click: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_hover: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_move: impl for<'a> Fn(&'a CallbackContext, Vec2) + Clone + 'static,
    context: &mut WidgetContext,
) -> FragmentInner {
    FragmentInner::Leaf {
        layout: Box::new(Maximal),
        render_object: RenderObject::Input {
            key: context.widget_local.key,
            on_click: Arc::new(on_click),
            on_hover: Arc::new(on_hover),
            on_move: Arc::new(on_move),
        },
    }
}


#[widget(on_click = (| _context, _clicked | {}), on_hover = (|_context, _hovered| {}), on_move = (|_context, _position| {}))]
pub fn input(
    children: Fragment,
    on_click: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_hover: impl for<'a> Fn(&'a CallbackContext, bool) + Clone + 'static,
    on_move: impl for<'a> Fn(&'a CallbackContext, Vec2) + Clone + 'static,
    context: &mut WidgetContext,
) -> Fragment {
    rsx! {
        <stack>
            <fragment>
                {Some(children)}
            </fragment>
            <positioned>
                <input_leaf on_click = on_click on_hover = on_hover on_move = on_move />
            </positioned>
        </stack>
    }
}
