use crate::{fragment, positioned, stack};
use narui::{layout::Maximal, *};
use narui_macros::{rsx, widget};
use std::sync::Arc;

#[widget]
pub fn input_leaf(
    #[default] on_click: impl for<'a> Fn(&'a CallbackContext, bool, Vec2, Vec2) + Clone + 'static,
    #[default] on_hover: impl for<'a> Fn(&'a CallbackContext, bool, Vec2, Vec2) + Clone + 'static,
    #[default] on_move: impl for<'a> Fn(&'a CallbackContext, Vec2, Vec2) + Clone + 'static,
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


#[widget]
pub fn input(
    children: Fragment,
    #[default] on_click: impl for<'a> Fn(&'a CallbackContext, bool, Vec2, Vec2) + Clone + 'static,
    #[default] on_hover: impl for<'a> Fn(&'a CallbackContext, bool, Vec2, Vec2) + Clone + 'static,
    #[default] on_move: impl for<'a> Fn(&'a CallbackContext, Vec2, Vec2) + Clone + 'static,
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
