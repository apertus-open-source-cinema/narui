use crate::{api::RenderObject, layout::PositionedRenderObject};
use winit::event::WindowEvent;

pub struct InputHandler {}
impl InputHandler {
    pub fn new() -> Self { InputHandler {} }
    pub fn handle_input(
        &mut self,
        event: WindowEvent,
        render_objects: Vec<PositionedRenderObject>,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                for render_object in render_objects {
                    if let RenderObject::Input { hover: Some(hover), .. } =
                        render_object.render_object
                    {
                        hover.set(render_object.rect.contains(position.into()));
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {}
            _ => {}
        }
    }
}
