use crate::heart::{RenderObject::Input, *};
use hashbrown::HashMap;
use winit::event::{ElementState, MouseButton, WindowEvent};

#[derive(Default)]
pub struct InputState {
    clicked: bool,
    hover: bool,
}

#[derive(Default)]
pub struct InputHandler {
    cursor_position: Vec2,
    cursor_moved: bool,

    cursor_pressed: bool,
    cursor_released: bool,

    input_states: HashMap<Key, InputState>,
}
impl InputHandler {
    pub fn new() -> Self { Default::default() }
    pub fn enqueue_input(&mut self, event: WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = position.into();
                self.cursor_moved = true;
                true
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.cursor_pressed = true;
                true
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.cursor_released = true;
                true
            }
            _ => false,
        }
    }
    pub fn handle_input(
        &mut self,
        render_objects: Vec<PositionedRenderObject>,
        context: Context,
    ) -> bool {
        if !self.cursor_moved && !self.cursor_pressed && !self.cursor_released {
            return false;
        }

        let mut updated = false;
        for render_object in render_objects.clone() {
            if let Input { on_hover, on_move, on_click } = render_object.clone().render_object {
                let input_state =
                    self.input_states.entry(render_object.key).or_insert(Default::default());
                if self.cursor_moved {
                    let is_hover = render_object.rect.contains(self.cursor_position);
                    if input_state.hover != is_hover {
                        on_hover(context.clone(), is_hover);
                        input_state.hover = is_hover;
                        updated = true;
                    }
                    if input_state.clicked || is_hover {
                        on_move(context.clone(), self.cursor_position - render_object.rect.pos);
                        updated = true;
                    }
                }
                if self.cursor_pressed {
                    if render_object.rect.contains(self.cursor_position) {
                        input_state.clicked = true;
                        on_click(context.clone(), true);
                        updated = true;
                    }
                }
                if self.cursor_released {
                    if input_state.clicked {
                        input_state.clicked = false;
                        on_click(context.clone(), false);
                        updated = true;
                    }
                }
            }
        }

        self.cursor_moved = false;
        self.cursor_pressed = false;
        self.cursor_released = false;

        updated
    }
}
