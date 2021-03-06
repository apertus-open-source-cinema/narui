/*
use crate::{eval::layout::PositionedElement, RenderObject};
use std::sync::Arc;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
    render_pass::RenderPass,
};
use crate::eval::layout::RenderObjectOrSubPass;

pub struct RawRenderer {
    render_pass: Arc<RenderPass>,
}
impl RawRenderer {
    pub fn new(render_pass: Arc<RenderPass>) -> Self { Self { render_pass } }
    pub fn render<'a>(
        &mut self,
        buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        dynamic_state: &DynamicState,
        dimensions: &[u32; 2],
        render_object: &PositionedElement<'a>,
    ) {
        if let RenderObjectOrSubPass::RenderObject(RenderObject::Raw { render_fn }) = &render_object.element {
            render_fn(
                self.render_pass.clone(),
                buffer_builder,
                dynamic_state,
                render_object.rect,
                (*dimensions).into(),
            );
        }
    }
}
*/
