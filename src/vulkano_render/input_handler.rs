use crate::heart::*;
use winit::event::{ElementState, MouseButton, WindowEvent};

pub struct InputHandler {
    left_mouse: bool,
    cursor_position: Vec2,
    click_start_position: Vec2,
}
impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            left_mouse: false,
            cursor_position: Vec2::zero(),
            click_start_position: Vec2::zero(),
        }
    }
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
                MouseButton::Left => {
                    let state = matches!(state, ElementState::Pressed);
                    if state {
                        self.click_start_position = self.cursor_position;
                    }
                    self.left_mouse = state
                }
                _ => {}
            },
            _ => {}
        }

        for render_object in render_objects {
            if let RenderObject::Input { hover: Some(hover), .. } =
                render_object.clone().render_object
            {
                hover.set(render_object.rect.contains(self.cursor_position.into()));
            }
            if let RenderObject::Input { click: Some(click), .. } =
                render_object.clone().render_object
            {
                click.set(
                    render_object.rect.contains(self.click_start_position.into())
                        && self.left_mouse,
                );
            }
        }
    }
}
