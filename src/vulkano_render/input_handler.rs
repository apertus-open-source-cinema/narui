use crate::heart::{RenderObject::Input, *};
use hashbrown::HashMap;
use winit::event::{ElementState, MouseButton, WindowEvent};

#[derive(Default)]
pub struct InputState {
    clicked: bool,
    hover: bool,
}

pub struct InputHandler {
    cursor_position: Vec2,
    input_states: HashMap<Key, InputState>,
}
impl InputHandler {
    pub fn new() -> Self {
        InputHandler { cursor_position: Vec2::zero(), input_states: Default::default() }
    }
    pub fn handle_input(
        &mut self,
        event: WindowEvent,
        render_objects: Vec<PositionedRenderObject>,
        context: Context,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = position.into();

                for render_object in render_objects.clone() {
                    if let Input { on_hover, on_move, on_click: _ } =
                        render_object.clone().render_object
                    {
                        let input_state = self
                            .input_states
                            .entry(render_object.key)
                            .or_insert(Default::default());
                        let is_hover = render_object.rect.contains(self.cursor_position);
                        if input_state.hover != is_hover {
                            on_hover(context.clone(), is_hover);
                            input_state.hover = is_hover;
                        }
                        if input_state.clicked || is_hover {
                            on_move(context.clone(), self.cursor_position - render_object.rect.pos);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => match state {
                ElementState::Pressed => {
                    for render_object in render_objects.clone() {
                        let input_state = self
                            .input_states
                            .entry(render_object.key)
                            .or_insert(Default::default());
                        if let Input { on_click, .. } = render_object.clone().render_object {
                            if render_object.rect.contains(self.cursor_position) {
                                input_state.clicked = true;
                                on_click(context.clone(), true);
                            }
                        }
                    }
                }
                ElementState::Released => {
                    for render_object in render_objects.clone() {
                        let input_state = self
                            .input_states
                            .entry(render_object.key)
                            .or_insert(Default::default());
                        if let Input { on_click, .. } = render_object.clone().render_object {
                            if input_state.clicked {
                                input_state.clicked = false;
                                on_click(context.clone(), false);
                            }
                        }
                    }
                }
            },
            _e => { /*dbg!(_e);*/ }
        }
    }
}
