use super::VulkanContext;
use crate::{heart::*, hooks::ContextListenable};
use hashbrown::HashMap;
use lyon::{
    lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
    tessellation::{path::Path, FillVertexConstructor},
};
use std::{sync::Arc, mem};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    descriptor::PipelineLayoutAbstract,
    device::Device,
    framebuffer::{RenderPassAbstract, Subpass},
    pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
};
use lyon::lyon_tessellation::StrokeVertexConstructor;
use lyon::tessellation::{StrokeVertex, StrokeTessellator, StrokeOptions};
use std::mem::size_of;
use std::ops::Deref;
use lyon::tessellation::path::path::Builder;
use lyon::tessellation::path::builder::PathBuilder;
use lyon::algorithms::path::geom::{rect};
use lyon::algorithms::path::Winding;
use stretch::geometry::Size;


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
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}
vulkano::impl_vertex!(Vertex, position, color);


pub struct LyonRenderer {
    device: Arc<Device>,
    pipeline: Arc<
        GraphicsPipeline<
            SingleBufferDefinition<Vertex>,
            Box<dyn PipelineLayoutAbstract + Send + Sync>,
            Arc<dyn RenderPassAbstract + Send + Sync>,
        >,
    >,
    fill_tessellator: FillTessellator,
    fill_cache: HashMap<(usize, (i64, i64)), VertexBuffers<Vec2, u16>>,
    stroke_tessellator: StrokeTessellator,
    stroke_cache: HashMap<(usize, (i64, i64), [u8; size_of::<StrokeOptions>()]), VertexBuffers<Vec2, u16>>,
}
impl LyonRenderer {
    pub fn new(render_pass: Arc<dyn RenderPassAbstract + Send + Sync>) -> Self {
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
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        Self {
            pipeline,
            device,

            fill_tessellator: FillTessellator::new(),
            fill_cache: HashMap::new(),

            stroke_tessellator: StrokeTessellator::new(),
            stroke_cache: HashMap::new(),
        }
    }
    pub fn render(
        &mut self,
        buffer_builder: &mut AutoCommandBufferBuilder,
        dynamic_state: &DynamicState,
        dimensions: &[u32; 2],
        render_objects: Vec<PositionedRenderObject>,
        context: Context,
    ) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut last_index = 0;

        for render_object in render_objects {
            let (buffer, color) = match render_object.render_object {
                RenderObject::FillPath { path_gen, color } => {
                    let color = [color.red, color.green, color.blue, color.alpha];
                    let path_gen = context.listen(path_gen);
                    let path_gen_key = path_gen.as_ref() as *const _ as *const usize as usize;
                    let buffer = self.fill_tessellate_with_cache(
                        path_gen, render_object.rect.size, path_gen_key,
                    );
                    (buffer, color)
                },
                RenderObject::StrokePath { path_gen, color, stroke_options } => {
                    let color = [color.red, color.green, color.blue, color.alpha];
                    let path_gen = context.listen(path_gen);
                    let path_gen_key = path_gen.as_ref() as *const _ as *const usize as usize;
                    let buffer = self.stroke_tessellate_with_cache(
                        path_gen,
                        stroke_options,
                        render_object.rect.size,
                        path_gen_key,
                    );
                    (buffer, color)
                }
                RenderObject::DebugRect => {
                    let color = [1.0, 0.0, 0.0, 0.5];
                    let buffer = self.stroke_tessellate_with_cache(
                        (&|size: Size<f32>| {
                            let mut builder = Builder::new();
                            builder.add_rectangle(
                                &rect(0.0, 0.0, size.width, size.height),
                                Winding::Positive,
                            );
                            builder.build()
                        }) as &PathGenInner,
                        StrokeOptions::default(), render_object.rect.size,
                        0,
                    );
                    (buffer, color)
                }
                _ => { continue }
            };

            for point in &buffer.vertices {
                vertices
                    .push(Vertex { position: (*point + render_object.rect.pos).into(), color })
            }
            for index in &buffer.indices {
                indices.push(index + last_index);
            }
            last_index += buffer.vertices.len() as u16;
        }

        let vertex_buffer = CpuAccessibleBuffer::<[Vertex]>::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            vertices.into_iter(),
        )
        .unwrap();

        let index_buffer = CpuAccessibleBuffer::<[u16]>::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            indices.into_iter(),
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
                vec![],
            )
            .unwrap();
    }
    pub fn fill_tessellate_with_cache(
        &mut self,
        path_gen: impl Deref<Target=PathGenInner>,
        size: Vec2,
        path_gen_key: usize,
    ) -> &VertexBuffers<Vec2, u16> {
        struct VertexConstructor {}
        impl FillVertexConstructor<Vec2> for VertexConstructor {
            fn new_vertex(&mut self, vertex: FillVertex) -> Vec2 { vertex.position().into() }
        }

        let mut lyon_vertex_buffer: VertexBuffers<Vec2, u16> = VertexBuffers::new();
        let cache_key = (path_gen_key, Into::<(i64, i64)>::into(size));
        if self.fill_cache.get(&cache_key).is_none() {
            let path: Path = path_gen(size.into());
            let mut buffers_builder =
                BuffersBuilder::new(&mut lyon_vertex_buffer, VertexConstructor {});
            self.fill_tessellator
                .tessellate_path(path.as_slice(), &FillOptions::DEFAULT, &mut buffers_builder)
                .unwrap();
            self.fill_cache.insert(cache_key, lyon_vertex_buffer);
        }
        self.fill_cache.get(&cache_key).unwrap()
    }
    pub fn stroke_tessellate_with_cache(
        &mut self,
        path_gen: impl Deref<Target=PathGenInner>,
        stroke_options: StrokeOptions,
        size: Vec2,
        path_gen_key: usize,
    ) -> &VertexBuffers<Vec2, u16> {
        struct VertexConstructor {}
        impl StrokeVertexConstructor<Vec2> for VertexConstructor {
            fn new_vertex(&mut self, vertex: StrokeVertex) -> Vec2 { vertex.position().into() }
        }

        let mut lyon_vertex_buffer: VertexBuffers<Vec2, u16> = VertexBuffers::new();

        let cache_key = (
            path_gen_key,
            Into::<(i64, i64)>::into(size),
            unsafe {
                mem::transmute_copy::<StrokeOptions, [u8; size_of::<StrokeOptions>()]>(&stroke_options)
            },
        );

        if self.stroke_cache.get(&cache_key).is_none() {
            let path: Path = path_gen(size.into());
            let mut buffers_builder =
                BuffersBuilder::new(&mut lyon_vertex_buffer, VertexConstructor {});
            self.stroke_tessellator
                .tessellate_path(path.as_slice(), &stroke_options, &mut buffers_builder)
                .unwrap();
            self.stroke_cache.insert(cache_key, lyon_vertex_buffer);
        }
        self.stroke_cache.get(&cache_key).unwrap()
    }
}
