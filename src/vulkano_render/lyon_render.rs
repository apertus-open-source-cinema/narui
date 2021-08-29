use super::VulkanContext;
use crate::heart::*;

use lyon::{
    lyon_tessellation::{
        BuffersBuilder,
        FillTessellator,
        FillVertex,
        StrokeVertexConstructor,
        VertexBuffers,
    },
    tessellation::{FillVertexConstructor, StrokeOptions, StrokeTessellator, StrokeVertex},
};
use palette::{Pixel};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
    device::Device,
    pipeline::{
        depth_stencil::{Compare, DepthStencil},
        vertex::BuffersDefinition,
        GraphicsPipeline,
    },
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
            layout(location = 0) in vec3 position;
            layout(location = 1) in vec4 color;
            layout(location = 2) in vec2 clip_min;
            layout(location = 3) in vec2 clip_max;
            layout(location = 0) out vec4 color_frag;
            layout(location = 1) out vec2 clip_min_out;
            layout(location = 2) out vec2 clip_max_out;
            void main() {
                color_frag = color;
                clip_min_out = clip_min;
                clip_max_out = clip_max;

                vec2 zero_to_one = position.xy / vec2(params.width, params.height);
                gl_Position = vec4(vec2(zero_to_one * 2. - vec2(1.)), position.z, 1.0);
            }
        "
    }
}
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(location = 0) in vec4 color_frag;
            layout(location = 1) in vec2 clip_min;
            layout(location = 2) in vec2 clip_max;
            layout(location = 0) out vec4 f_color;

            void main() {
                if (gl_FragCoord.x < clip_min.x || gl_FragCoord.y < clip_min.y
                 || gl_FragCoord.x > clip_max.x || gl_FragCoord.y > clip_max.y) {
                    discard;
                }

                f_color = color_frag;
            }
        "
    }
}

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
    clip_min: [f32; 2],
    clip_max: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position, color, clip_min, clip_max);

#[derive(Default)]
pub struct LyonRendererState(VertexBuffers<Vertex, u32>);

pub struct ColoredBuffersBuilder<'a> {
    vertex_buffers: &'a mut VertexBuffers<Vertex, u32>,
    pos: Vec2,
    z_index: f32,
    clip_min: [f32; 2],
    clip_max: [f32; 2],
}

pub struct PositionedColoredConstructor {
    position: Vec2,
    color: [f32; 4],
    z_index: f32,
    clip_min: [f32; 2],
    clip_max: [f32; 2],
}
impl FillVertexConstructor<Vertex> for PositionedColoredConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        let pos: Vec2 = vertex.position().into();
        let pos = pos + self.position;
        Vertex {
            position: [pos.x, pos.y, self.z_index],
            color: self.color,
            clip_min: self.clip_min,
            clip_max: self.clip_max,
        }
    }
}
impl StrokeVertexConstructor<Vertex> for PositionedColoredConstructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Vertex {
        let pos: Vec2 = vertex.position().into();
        let pos = pos + self.position;
        Vertex {
            position: [pos.x, pos.y, self.z_index],
            color: self.color,
            clip_min: self.clip_min,
            clip_max: self.clip_max,
        }
    }
}

impl<'a> ColoredBuffersBuilder<'a> {
    pub fn with_color(
        &mut self,
        color: Color,
    ) -> BuffersBuilder<Vertex, u32, PositionedColoredConstructor> {
        BuffersBuilder::new(
            &mut self.vertex_buffers,
            PositionedColoredConstructor {
                position: self.pos,
                color: color.into_raw::<[f32; 4]>(),
                z_index: self.z_index,
                clip_min: self.clip_min,
                clip_max: self.clip_max,
            },
        )
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
                .depth_stencil(DepthStencil {
                    depth_compare: Compare::LessOrEqual,
                    ..DepthStencil::simple_depth_test()
                })
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
    pub fn begin(&self) -> LyonRendererState { Default::default() }
    pub fn render<'a>(
        &mut self,
        state: &mut LyonRendererState,
        render_object: &PositionedRenderObject<'a>,
    ) {
        let clipping_rect = if let Some(clipping_rect) = render_object.clipping_rect {
            render_object.rect.clip(clipping_rect)
        } else {
            render_object.rect
        };

        let LyonRendererState(vertex_buffers) = state;
        match render_object.render_object {
            RenderObject::Path { path_gen } => {
                (path_gen)(
                    render_object.rect.size,
                    &mut self.fill_tessellator,
                    &mut self.stroke_tessellator,
                    ColoredBuffersBuilder {
                        vertex_buffers,
                        pos: render_object.rect.pos,
                        z_index: 1.0 - render_object.z_index as f32 / 65535.0,
                        clip_min: clipping_rect.near_corner().into(),
                        clip_max: clipping_rect.far_corner().into(),
                    },
                );
            }
            RenderObject::DebugRect => {
                let r = render_object.rect;
                self.stroke_tessellator
                    .tessellate_rectangle(
                        &lyon::math::rect(0.0, 0.0, r.size.x, r.size.y),
                        &StrokeOptions::default().with_line_width(2.0),
                        &mut ColoredBuffersBuilder {
                            vertex_buffers,
                            pos: render_object.rect.pos,
                            z_index: 0.0,
                            clip_min: [0., 0.],
                            clip_max: [10000., 10000.],
                        }
                        .with_color(Color::new(1.0, 0.0, 0.0, 0.25)),
                    )
                    .unwrap();
            }
            _ => {}
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

        let index_buffer = CpuAccessibleBuffer::<[u32]>::from_iter(
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
