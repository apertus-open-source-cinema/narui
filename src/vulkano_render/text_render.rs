use crate::{
    api::RenderObject,
    layout::PositionedRenderObject,
    types::Color,
    vulkano_render::VulkanContext,
};
use glyph_brush::{
    ab_glyph::{FontArc, PxScale},
    BrushAction,
    BrushError,
    GlyphBrush,
    GlyphBrushBuilder,
    GlyphVertex,
    Section,
    Text,
};
use notosans::REGULAR_TTF as FONT;
use std::{iter::repeat, sync::Arc};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    descriptor::{
        descriptor_set::{
            PersistentDescriptorSet,
            PersistentDescriptorSetImg,
            PersistentDescriptorSetSampler,
        },
        PipelineLayoutAbstract,
    },
    device::{Device, Queue},
    format::Format,
    framebuffer::{RenderPassAbstract, Subpass},
    image::{
        view::ImageView,
        ImageCreateFlags,
        ImageDimensions,
        ImageLayout,
        ImageUsage,
        ImmutableImage,
        MipmapsCount,
    },
    pipeline::{
        vertex::{OneVertexOneInstanceDefinition, SingleBufferDefinition},
        GraphicsPipeline,
    },
    sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
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

            layout(location = 0) in uint id;

            layout(location = 1) in vec2 pos_min;
            layout(location = 2) in vec2 pos_max;
            layout(location = 3) in vec2 tex_min;
            layout(location = 4) in vec2 tex_max;
            layout(location = 5) in vec4 color;

            layout(location = 0) out vec2 tex_frag;
            layout(location = 1) out vec4 color_frag;

            void main() {
                color_frag = color;

                vec2 pos = vec2(0.0);
                switch (id) {
                    case 0:
                        pos = pos_min;
                        tex_frag = tex_min;
                        break;
                    case 1:
                        pos = vec2(pos_max.x, pos_min.y);
                        tex_frag = vec2(tex_max.x, tex_min.y);
                        break;
                    case 2:
                        pos = vec2(pos_min.x, pos_max.y);
                        tex_frag = vec2(tex_min.x, tex_max.y);
                        break;
                    case 3:
                        pos = pos_max;
                        tex_frag = tex_max;
                        break;
                }
                gl_Position = vec4((pos / (vec2(params.width, params.height) / 2.) - vec2(1.)), 0.0, 1.0);
            }
        "
    }
}
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            layout(set = 0, binding = 0) uniform sampler2D tex;
            layout(location = 0) in vec2 tex_frag;
            layout(location = 1) in vec4 color_frag;

            layout(location = 0) out vec4 f_color;

            void main() {
                float alpha = texture(tex, tex_frag).r;
                if (alpha <= 0.0) {
                    discard;
                }
                f_color = color_frag;
                f_color.a *= alpha;
            }
        "
    }
}

#[derive(Default, Debug, Clone)]
struct Vertex {
    id: u32,
}
vulkano::impl_vertex!(Vertex, id);

#[derive(Default, Debug, Clone)]
struct InstanceData {
    pos_min: [f32; 2],
    pos_max: [f32; 2],
    tex_min: [f32; 2],
    tex_max: [f32; 2],
    color: [f32; 4],
}
vulkano::impl_vertex!(InstanceData, pos_min, pos_max, tex_min, tex_max, color);

pub struct TextRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: std::sync::Arc<
        GraphicsPipeline<
            OneVertexOneInstanceDefinition<Vertex, InstanceData>,
            Box<dyn PipelineLayoutAbstract + Send + Sync>,
            std::sync::Arc<dyn RenderPassAbstract + Send + Sync>,
        >,
    >,
    glyph_brush: GlyphBrush<InstanceData>,
    quad_vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    instance_data_buffer: Arc<CpuAccessibleBuffer<[InstanceData]>>,
    sampler: Arc<Sampler>,
    descriptor_set: Arc<
        PersistentDescriptorSet<(
            ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage<Format>>>>>),
            PersistentDescriptorSetSampler,
        )>,
    >,
}
impl TextRenderer {
    pub fn new(render_pass: Arc<dyn RenderPassAbstract + Send + Sync>, queue: Arc<Queue>) -> Self {
        let device = VulkanContext::get().device;

        let vs = vertex_shader::Shader::load(device.clone()).unwrap();
        let fs = fragment_shader::Shader::load(device.clone()).unwrap();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input(OneVertexOneInstanceDefinition::<Vertex, InstanceData>::new())
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_strip()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .blend_alpha_blending()
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let droid_sans = FontArc::try_from_slice(FONT).unwrap();
        let glyph_brush = GlyphBrushBuilder::using_font(droid_sans).build();

        let quad_vertex_buffer = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            [Vertex { id: 0 }, Vertex { id: 1 }, Vertex { id: 2 }, Vertex { id: 3 }]
                .iter()
                .cloned(),
        )
        .unwrap();

        let instance_data_buffer = CpuAccessibleBuffer::<[InstanceData]>::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            (vec![]).into_iter(),
        )
        .unwrap();

        let (image, future) = ImmutableImage::from_iter(
            vec![0u8].into_iter(),
            ImageDimensions::Dim2d { width: 1, height: 1, array_layers: 1 },
            MipmapsCount::One,
            Format::R8Unorm,
            queue.clone(),
        )
        .unwrap();
        let texture = ImageView::new(image).unwrap();

        let sampler = Sampler::new(
            device.clone(),
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0,
            1.0,
            0.0,
            0.0,
        )
        .unwrap();

        let layout = pipeline.layout().descriptor_set_layout(0).unwrap();
        let descriptor_set = Arc::new(
            PersistentDescriptorSet::start(layout.clone())
                .add_sampled_image(texture.clone(), sampler.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        Self {
            pipeline,
            device,
            glyph_brush,
            quad_vertex_buffer,
            instance_data_buffer,
            descriptor_set,
            sampler,
            queue,
        }
    }
    pub fn render(
        &mut self,
        buffer_builder: &mut AutoCommandBufferBuilder,
        dynamic_state: &DynamicState,
        dimensions: &[u32; 2],
        render_objects: Vec<PositionedRenderObject>,
    ) {
        for render_object in render_objects {
            if let RenderObject::Text { text, size, color } = render_object.render_object {
                self.glyph_brush.queue(
                    Section::default()
                        .add_text(
                            Text::new(&*text).with_color(color).with_scale(PxScale::from(size)),
                        )
                        .with_screen_position((render_object.position.x, render_object.position.y))
                        .with_bounds((render_object.size.width, render_object.size.height)),
                );
            }
        }

        let mut texture_upload = None;
        match self.glyph_brush.process_queued(
            |rect, tex_data| {
                texture_upload = Some((tex_data.to_vec(), rect));
            },
            |vertex_data| InstanceData {
                pos_min: [vertex_data.pixel_coords.min.x, vertex_data.pixel_coords.min.y],
                pos_max: [vertex_data.pixel_coords.max.x, vertex_data.pixel_coords.max.y],
                tex_min: [vertex_data.tex_coords.min.x, vertex_data.tex_coords.min.y],
                tex_max: [vertex_data.tex_coords.max.x, vertex_data.tex_coords.max.y],
                color: vertex_data.extra.color,
            },
        ) {
            Ok(BrushAction::Draw(vertices)) => {
                self.instance_data_buffer = CpuAccessibleBuffer::<[InstanceData]>::from_iter(
                    self.device.clone(),
                    BufferUsage::all(),
                    false,
                    vertices.into_iter(),
                )
                .unwrap();
            }
            Ok(BrushAction::ReDraw) => {}
            Err(BrushError::TextureTooSmall { suggested: (w, h) }) => {
                // we always create a new texture so we do not need to handle this case in a
                // special way
                self.glyph_brush.resize_texture(w, h);
            }
        }
        if let Some((mut tex_data, rect)) = texture_upload {
            dbg!(rect);
            let (width, height) = self.glyph_brush.texture_dimensions();
            tex_data.append(
                &mut repeat(0u8).take((width * height) as usize - tex_data.len()).collect(),
            );
            let (image, future) = ImmutableImage::from_iter(
                tex_data.iter().cloned(),
                ImageDimensions::Dim2d { width, height, array_layers: 1 },
                MipmapsCount::One,
                Format::R8Unorm,
                self.queue.clone(),
            )
            .unwrap();
            let texture = ImageView::new(image).unwrap();

            let layout = self.pipeline.layout().descriptor_set_layout(0).unwrap();
            self.descriptor_set = Arc::new(
                PersistentDescriptorSet::start(layout.clone())
                    .add_sampled_image(texture.clone(), self.sampler.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            );
        }

        let push_constants =
            vertex_shader::ty::PushConstantData { width: dimensions[0], height: dimensions[1] };
        buffer_builder
            .draw(
                self.pipeline.clone(),
                &dynamic_state,
                (self.quad_vertex_buffer.clone(), self.instance_data_buffer.clone()),
                self.descriptor_set.clone(),
                push_constants,
                vec![],
            )
            .unwrap();
    }
}
