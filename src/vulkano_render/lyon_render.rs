use super::VulkanContext;
use crate::heart::*;
use hashbrown::HashMap;
use lyon::{
    algorithms::path::{geom::rect, Winding},
    lyon_tessellation::{
        BuffersBuilder,
        FillOptions,
        FillTessellator,
        FillVertex,
        StrokeVertexConstructor,
        VertexBuffers,
    },
    tessellation::{
        path::{builder::PathBuilder, path::Builder, Path},
        FillVertexConstructor,
        StrokeOptions,
        StrokeTessellator,
        StrokeVertex,
    },
};
use palette::Pixel;
use std::{mem, mem::size_of, ops::Deref, sync::Arc};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
    device::Device,
    pipeline::{vertex::BuffersDefinition, GraphicsPipeline},
    render_pass::{RenderPass, Subpass},
};


mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            layout(push_constant) uniform PushConstantData {
                uint width;
                uint height;
            } params;
            layout(location = 0) in vec2 position;
            layout(location = 1) in vec4 color;
            layout(location = 0) out vec4 color_frag;
            void main() {
                color_frag = color;
                vec2 zero_to_one = position / vec2(params.width, params.height);
                gl_Position = vec4(vec2(zero_to_one * 2. - vec2(1.)), 0.0, 1.0);
            }
        "
    }
}
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) out vec4 f_color;
            layout(location = 0) in vec4 color_frag;

            void main() {
                f_color = color_frag;
            }
        "
    }
}

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}
vulkano::impl_vertex!(Vertex, position, color);

#[derive(Default)]
pub struct LyonRendererState(VertexBuffers<Vertex, u16>);

pub struct ColoredBuffersBuilder<'a> {
    vertex_buffers: &'a mut VertexBuffers<Vertex, u16>,
    pos: Vec2
}

pub struct PositionedColoredConstructor(Vec2, [f32; 4]);

impl FillVertexConstructor<Vertex> for PositionedColoredConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        let pos: Vec2 = vertex.position().into();
        Vertex {
            position: (pos + self.0).into(), color: self.1
        }
    }
}

impl StrokeVertexConstructor<Vertex> for PositionedColoredConstructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex {
        let pos: Vec2 = vertex.position().into();
        Vertex {
            position: (pos + self.0).into(), color: self.1
        }
    }
}

impl<'a> ColoredBuffersBuilder<'a> {
    pub fn with_color(&mut self, color: Color) -> BuffersBuilder<Vertex, u16, PositionedColoredConstructor> {
        BuffersBuilder::new(&mut self.vertex_buffers, PositionedColoredConstructor(self.pos, color.into_raw::<[f32; 4]>().into()))
    }
}

pub struct LyonRenderer {
    device: Arc<Device>,
    pipeline: Arc<GraphicsPipeline<BuffersDefinition>>,
    fill_tessellator: FillTessellator,
    stroke_tessellator: StrokeTessellator,
}
impl LyonRenderer {
    pub fn new(render_pass: Arc<RenderPass>) -> Self {
        let device = VulkanContext::get().device;

        let vs = vertex_shader::Shader::load(device.clone()).unwrap();
        let fs = fragment_shader::Shader::load(device.clone()).unwrap();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .blend_alpha_blending()
                .render_pass(Subpass::from(render_pass, 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        Self {
            pipeline,
            device,

            fill_tessellator: FillTessellator::new(),
            stroke_tessellator: StrokeTessellator::new(),
        }
    }
    pub fn begin(&self) -> LyonRendererState {
        Default::default()
    }
    pub fn render<'a>(
        &mut self,
        state: &mut LyonRendererState,
        render_object: &PositionedRenderObject<'a>,
    ) {
        let LyonRendererState(vertex_buffers) = state;
        match render_object.render_object {
            RenderObject::Path { path_gen } => {
                (path_gen)(
                    render_object.rect.size.into(),
                    &mut self.fill_tessellator,
                    &mut self.stroke_tessellator,
                    ColoredBuffersBuilder {
                        vertex_buffers,
                        pos: render_object.rect.pos
                    }
                );
            }
            RenderObject::DebugRect => {
                let r = render_object.rect;
                self.stroke_tessellator.tessellate_rectangle(
                    &lyon::math::rect(r.pos.x, r.pos.y,r.size.x, r.size.y),
                    &StrokeOptions::default(),
                    &mut ColoredBuffersBuilder {
                        vertex_buffers,
                        pos: render_object.rect.pos
                    }.with_color(Color::new(1.0, 0.0, 0.0, 0.5)));
            }
            _ => {},
        };
    }
    pub fn finish(
        &mut self,
        state: LyonRendererState,
        buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        dynamic_state: &DynamicState,
        dimensions: &[u32; 2],
    ) {
        let vertex_buffer = CpuAccessibleBuffer::<[Vertex]>::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            state.0.vertices.into_iter(),
        )
        .unwrap();

        let index_buffer = CpuAccessibleBuffer::<[u16]>::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            state.0.indices.into_iter(),
        )
        .unwrap();

        let push_constants =
            vertex_shader::ty::PushConstantData { width: dimensions[0], height: dimensions[1] };
        buffer_builder
            .draw_indexed(
                self.pipeline.clone(),
                dynamic_state,
                vertex_buffer,
                index_buffer,
                (),
                push_constants,
            )
            .unwrap();
    }
}
