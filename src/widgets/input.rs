use crate::*;
use std::sync::Arc;

#[widget(on_click = (|_context, _clicked| {}), on_hover = (|_context, _hovered| {}), on_move = (|_context, _position| {}), style = Default::default())]
pub fn input(
    on_click: impl Fn(Context, bool) + Clone + Sync + Send + 'static,
    on_hover: impl Fn(Context, bool) + Clone + Sync + Send + 'static,
    on_move: impl Fn(Context, Vec2) + Clone + Sync + Send + 'static,
    style: Style,
    children: Fragment,
    context: Context,
) -> Fragment {
    Fragment {
        key: context.widget_local.key,
        children: children.into(),
        layout_object: Some(LayoutObject {
            style,
            measure_function: None,
            render_objects: vec![(
                KeyPart::RenderObject(0),
                RenderObject::Input {
                    on_click: Arc::new(on_click),
                    on_hover: Arc::new(on_hover),
                    on_move: Arc::new(on_move),
                },
            )],
        }),
    }
}
