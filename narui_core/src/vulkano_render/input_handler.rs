use crate::{
    eval::layout::{LayoutTree, Layouter, Physical, ScaleFactor},
    geom::{Rect, Vec2},
    CallbackContext,
    Key,
    RenderObject,
};
use freelist::Idx;
use hashbrown::HashMap;
use winit::event::{ElementState, MouseButton, WindowEvent};

#[derive(Default)]
pub struct InputState {
    clicked: bool,
    hover: bool,
}

#[derive(Default)]
pub struct InputHandler {
    cursor_position: Physical<Vec2>,
    cursor_moved: bool,

    cursor_pressed: bool,
    cursor_released: bool,

    input_states: HashMap<Key, InputState, ahash::RandomState>,
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
        input_render_object: &[(Idx, Option<Physical<Rect>>)],
        layouter: &Layouter,
        context: CallbackContext,
        scale_factor: ScaleFactor,
    ) -> bool {
        if !self.cursor_moved && !self.cursor_pressed && !self.cursor_released {
            return false;
        }

        let mut updated = false;
        for (key, clipping_rect) in input_render_object {
            let (rect, obj) = layouter.get_positioned_physical(*key, scale_factor);
            let rect = if let Some(clipping_rect) = clipping_rect {
                rect.map(|rect| clipping_rect.map(|clipping_rect| rect.clip(clipping_rect))).into()
            } else {
                rect
            };

            let dist = self
                .cursor_position
                .map(|pos| pos - rect.unwrap_physical().pos)
                .to_logical(scale_factor);
            let pos = self.cursor_position.to_logical(scale_factor);

            if let Some(RenderObject::Input { key, on_hover, on_move, on_click }) = obj {
                let input_state = self.input_states.entry(*key).or_insert(Default::default());
                let cursor_position = self.cursor_position;
                let is_hover = rect
                    .map(|rect| rect.contains(cursor_position.unwrap_physical()))
                    .unwrap_physical();
                if self.cursor_moved {
                    if input_state.hover != is_hover {
                        on_hover(&context, is_hover, dist, pos);
                        input_state.hover = is_hover;
                        updated = true;
                    }
                    if input_state.clicked || is_hover {
                        on_move(&context, dist, pos);
                        updated = true;
                    }
                }
                if self.cursor_pressed && is_hover {
                    input_state.clicked = true;
                    on_click(&context, true, dist, pos);
                    updated = true;
                }
                if self.cursor_released && (input_state.clicked || is_hover) {
                    input_state.clicked = false;
                    on_click(&context, false, dist, pos);
                    updated = true;
                }
            }
        }

        self.cursor_moved = false;
        self.cursor_pressed = false;
        self.cursor_released = false;

        updated
    }
}
