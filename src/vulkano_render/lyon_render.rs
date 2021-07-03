use super::VulkanContext;
use crate::{heart::*, hooks::ContextListenable};
use hashbrown::HashMap;
use lyon::{
    lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
    tessellation::{path::Path, FillVertexConstructor},
};
use std::{any::Any, sync::Arc};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    descriptor::PipelineLayoutAbstract,
    device::Device,
    framebuffer::{RenderPassAbstract, Subpass},
    pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
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
    fill_tesselator: FillTessellator,
    cache: HashMap<(*const Box<dyn Any + Send + Sync>, (i64, i64)), VertexBuffers<Vec2, u16>>,
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
        let fill_tesselator = FillTessellator::new();

        Self { pipeline, device, fill_tesselator, cache: HashMap::new() }
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
            if let RenderObject::Path { path_gen, color } = render_object.render_object {
                let color = [color.red, color.green, color.blue, color.alpha];
                let buffer =
                    self.tesselate_with_cache(path_gen, render_object.rect.size, context.clone());
                for point in &buffer.vertices {
                    vertices
                        .push(Vertex { position: (*point + render_object.rect.pos).into(), color })
                }
                for index in &buffer.indices {
                    indices.push(index + last_index);
                }
                last_index += buffer.vertices.len() as u16;
            }
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
    pub fn tesselate_with_cache(
        &mut self,
        path_gen: PathGen,
        size: Vec2,
        context: Context,
    ) -> &VertexBuffers<Vec2, u16> {
        struct VertexConstructor {}
        impl FillVertexConstructor<Vec2> for VertexConstructor {
            fn new_vertex(&mut self, vertex: FillVertex) -> Vec2 { vertex.position().into() }
        }

        let mut lyon_vertex_buffer: VertexBuffers<Vec2, u16> = VertexBuffers::new();
        let cache_key =
            (Arc::as_ptr(&context.listen(path_gen)) as *const _, Into::<(i64, i64)>::into(size));
        if self.cache.get(&cache_key).is_none() {
            let path: Path = context.listen(path_gen)(size.into());
            let mut buffers_builder =
                BuffersBuilder::new(&mut lyon_vertex_buffer, VertexConstructor {});
            self.fill_tesselator
                .tessellate_path(path.as_slice(), &FillOptions::DEFAULT, &mut buffers_builder)
                .unwrap();
            self.cache.insert(cache_key, lyon_vertex_buffer);
        }
        self.cache.get(&cache_key).unwrap()
    }
}
