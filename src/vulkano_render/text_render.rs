use super::VulkanContext;
use crate::heart::*;
use glyph_brush::{
    ab_glyph::{FontArc, PxScale},
    BrushAction,
    BrushError,
    GlyphBrush,
    GlyphBrushBuilder,
    Section,
    Text,
};

use lazy_static::lazy_static;
use palette::Pixel;
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer},
    descriptor_set::persistent::{
        PersistentDescriptorSet,
        PersistentDescriptorSetImg,
        PersistentDescriptorSetSampler,
    },
    device::{Device, Queue},
    format::Format,
    image::{view::ImageView, ImageDimensions, ImmutableImage, MipmapsCount},
    pipeline::{vertex::BuffersDefinition, GraphicsPipeline, GraphicsPipelineAbstract},
    render_pass::{RenderPass, Subpass},
    sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
};

lazy_static! {
    pub static ref FONT: FontArc = FontArc::try_from_slice(notosans::REGULAR_TTF).unwrap();
}

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
                if (alpha == 0.0) {
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
    pipeline: std::sync::Arc<GraphicsPipeline<BuffersDefinition>>,
    glyph_brush: GlyphBrush<InstanceData>,
    quad_vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    instance_data_buffer: Arc<CpuAccessibleBuffer<[InstanceData]>>,
    sampler: Arc<Sampler>,
    descriptor_set: Arc<
        PersistentDescriptorSet<(
            ((), PersistentDescriptorSetImg<Arc<ImageView<Arc<ImmutableImage>>>>),
            PersistentDescriptorSetSampler,
        )>,
    >,
    texture_bytes: Vec<u8>,
}
impl TextRenderer {
    pub fn new(render_pass: Arc<RenderPass>, queue: Arc<Queue>) -> Self {
        let device = VulkanContext::get().device;

        let vs = vertex_shader::Shader::load(device.clone()).unwrap();
        let fs = fragment_shader::Shader::load(device.clone()).unwrap();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input(
                    BuffersDefinition::new().vertex::<Vertex>().instance::<InstanceData>(),
                )
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_strip()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .blend_alpha_blending()
                .render_pass(Subpass::from(render_pass, 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let glyph_brush = GlyphBrushBuilder::using_font(FONT.clone()).build();

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

        let (image, _) = ImmutableImage::from_iter(
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

        let layout = pipeline.layout().descriptor_set_layouts()[0].clone();
        let descriptor_set = Arc::new(
            PersistentDescriptorSet::start(layout)
                .add_sampled_image(texture, sampler.clone())
                .unwrap()
                .build()
                .unwrap(),
        );
        let (w, h) = glyph_brush.texture_dimensions();
        let texture_bytes = vec![0u8; (w * h) as usize];

        Self {
            pipeline,
            device,
            glyph_brush,
            quad_vertex_buffer,
            instance_data_buffer,
            descriptor_set,
            sampler,
            queue,
            texture_bytes,
        }
    }
    pub fn render(
        &mut self,
        buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        dynamic_state: &DynamicState,
        dimensions: &[u32; 2],
        render_objects: Arc<Vec<PositionedRenderObject>>,
    ) {
        for render_object in render_objects.iter() {
            if let RenderObject::Text { text, size, color } = &render_object.render_object {
                self.glyph_brush.queue(
                    Section::default()
                        .add_text(
                            Text::new(&*text)
                                .with_color(color.into_format().into_raw::<[f32; 4]>())
                                .with_scale(PxScale::from(*size)),
                        )
                        .with_screen_position(Into::<(f32, f32)>::into(render_object.rect.pos))
                        .with_bounds(Into::<(f32, f32)>::into(
                            render_object.rect.size + Vec2::new(1., 1.),
                        )), // we seem to have some numerical instability issues here
                );
            }
        }

        let (width, height) = self.glyph_brush.texture_dimensions();
        let texture_bytes = &mut self.texture_bytes;
        let mut texture_upload = false;
        match self.glyph_brush.process_queued(
            |patch_rect, patch_data| {
                let patch_width = (patch_rect.max[0] - patch_rect.min[0]) as usize;
                for (y, line) in patch_data.chunks(patch_width).enumerate() {
                    let line_offset = (y + patch_rect.min[1] as usize) * width as usize
                        + patch_rect.min[0] as usize;
                    for (i, x) in (line_offset..line_offset + patch_width).enumerate() {
                        texture_bytes[x] = line[i];
                    }
                }
                texture_upload = true;
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
                /* //draw texture:
                vertices.push(InstanceData {
                    pos_min: [100., 100.],
                    pos_max: [500., 500.],
                    tex_min: [0., 0.],
                    tex_max: [1., 1.],
                    color: [1., 1., 1., 1.]
                });
                 */

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
                self.texture_bytes = vec![0u8; (w * h) as usize];
                self.glyph_brush.resize_texture(w, h);
            }
        }

        if texture_upload {
            let (image, _) = ImmutableImage::from_iter(
                self.texture_bytes.iter().cloned(),
                ImageDimensions::Dim2d { width, height, array_layers: 1 },
                MipmapsCount::One,
                Format::R8Unorm,
                self.queue.clone(),
            )
            .unwrap();
            let texture = ImageView::new(image).unwrap();

            let layout = self.pipeline.layout().descriptor_set_layouts()[0].clone();
            self.descriptor_set = Arc::new(
                PersistentDescriptorSet::start(layout)
                    .add_sampled_image(texture, self.sampler.clone())
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
                dynamic_state,
                (self.quad_vertex_buffer.clone(), self.instance_data_buffer.clone()),
                self.descriptor_set.clone(),
                push_constants,
            )
            .unwrap();
    }
}
