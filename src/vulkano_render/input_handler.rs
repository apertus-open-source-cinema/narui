use crate::heart::{RenderObject::Input, *};
use winit::event::{ElementState, MouseButton, WindowEvent};

pub struct InputHandler {
    cursor_position: Vec2,
}
impl InputHandler {
    pub fn new() -> Self { InputHandler { cursor_position: Vec2::zero() } }
    pub fn handle_input(
        &mut self,
        event: WindowEvent,
        render_objects: Vec<PositionedRenderObject>,
        context: Context,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = position.into();
            }
            WindowEvent::MouseInput { state, button, .. } => match button {
                MouseButton::Left => match state {
                    ElementState::Pressed => {
                        for render_object in render_objects.clone() {
                            if let Input { on_click, .. } = render_object.clone().render_object {
                                on_click(context.clone(), true);
                            }
                        }
                    }
                    ElementState::Released => {
                        for render_object in render_objects.clone() {
                            if let Input { on_click, .. } = render_object.clone().render_object {
                                on_click(context.clone(), false);
                            }
                        }
                    }
                },
                _ => {}
            },
            _e => { /*dbg!(_e);*/ }
        }

        for render_object in render_objects.clone() {
            if let Input { on_hover, on_move, on_click } = render_object.clone().render_object {
                let is_hover = render_object.rect.contains(self.cursor_position.into());
                on_hover(context.clone(), is_hover);
                if is_hover {
                    on_move(context.clone(), self.cursor_position - render_object.rect.pos);
                }
            }
        }
    }
}
