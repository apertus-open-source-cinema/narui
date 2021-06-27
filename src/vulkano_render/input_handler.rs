use crate::heart::{RenderObject::Input, *};
use winit::event::{ElementState, MouseButton, WindowEvent};
use narui_derive::get;

pub struct InputHandler {
    cursor_position: Vec2,
}
impl InputHandler {
    pub fn new() -> Self { InputHandler { cursor_position: Vec2::zero() } }
    pub fn handle_input(
        &mut self,
        event: WindowEvent,
        render_objects: Vec<PositionedRenderObject>,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = position.into();
            }
            WindowEvent::MouseInput { state, button, .. } => match button {
                MouseButton::Left => match state {
                    ElementState::Pressed => {
                        for render_object in render_objects.clone() {
                            if let Input { click, .. } = render_object.clone().render_object {
                                click.update(render_object.rect.contains(self.cursor_position));
                            }
                        }
                    }
                    ElementState::Released => {
                        for render_object in render_objects.clone() {
                            if let Input { click, .. } = render_object.clone().render_object {
                                click.update(false);
                            }
                        }
                    }
                },
                _ => {}
            },
            _e => { /*dbg!(_e);*/ }
        }

        for render_object in render_objects.clone() {
            if let Input { hover, position, click } = render_object.clone().render_object {
                hover.update(render_object.rect.contains(self.cursor_position.into()));

                if hover.get_sneaky() || click.get_sneaky() {
                    position.update(Some(self.cursor_position - render_object.rect.pos))
                } else {
                    position.update(None)
                }
            }
        }
    }
}
