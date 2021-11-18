use crate::{eval::layout::PositionedElement, geom::Rect, Dimension, RenderObject, Vec2};
use crevice::std430::AsStd430;


use crate::eval::layout::RenderObjectOrSubPass;
use palette::Pixel;
use std::sync::Arc;
use vulkano::{
    buffer::{BufferAccess, BufferUsage, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    descriptor_set::{persistent::PersistentDescriptorSet, DescriptorSet},
    device::{Device, Queue},
    image::{view::ImageView, ImmutableImage},
    pipeline::{
        blend::{AttachmentBlend, BlendFactor, BlendOp},
        depth_stencil::{CompareOp, DepthStencil},
        vertex::BuffersDefinition,
        viewport::Viewport,
        GraphicsPipeline,
        PipelineBindPoint,
    },
    render_pass::{RenderPass, Subpass},
    sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
    sync::GpuFuture,
};

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
            #version 450
            #extension GL_EXT_debug_printf: enable
            layout(push_constant) uniform PushConstantData {
                uint width;
                uint height;
                vec2 offset;
            } params;

            struct PerPrimitiveData {
                uint ty;
                float z_index;
                vec2 base_or_center;
                vec4 color;
                vec2 tex_base_or_half_size;
                vec2 tex_scale_or_border_radius_and_stroke_width;
                vec2 clip_min;
                vec2 clip_max;
            };

            layout(set = 0, binding = 0, std430) buffer readonly PrimitiveData { PerPrimitiveData data[]; } primitive_data;
            layout(location = 0) in vec2 pos;
            layout(location = 1) in uint primitive_index;

            layout(location = 0) out vec2 tex_or_rel_pos;
            layout(location = 1) out vec2 half_size;
            layout(location = 2) out vec4 color;
            layout(location = 3) out uint ty;
            layout(location = 4) out float inverted;
            layout(location = 5) out float border_radius;
            layout(location = 6) out float stroke_width;
            layout(location = 7) out vec2 clip_min;
            layout(location = 8) out vec2 clip_max;
            layout(location = 9) out float for_clipping;

            void main() {
                PerPrimitiveData data = primitive_data.data[primitive_index];
                color = data.color;
                ty = data.ty & 3;
                // debugPrintfEXT(\"pos = %f, %f, offset = %f, %f, color = %f, %f, %f, %f\", pos.x, pos.y, params.offset.x, params.offset.y, color.x, color.y, color.z, color.w);
                border_radius = data.tex_scale_or_border_radius_and_stroke_width.x;
                stroke_width = data.tex_scale_or_border_radius_and_stroke_width.y;
                half_size = data.tex_base_or_half_size;
                tex_or_rel_pos = vec2(0.0);
                clip_min = data.clip_min - params.offset;
                clip_max = data.clip_max - params.offset;
                inverted = float(ty == 3);
                for_clipping = float((data.ty & 4) > 0);

                // 0 is lyon
                // 1 is text
                // 2 and 3 is rounded rect
                if (ty == 1) {
                    tex_or_rel_pos = ((pos - data.base_or_center) * data.tex_scale_or_border_radius_and_stroke_width) + data.tex_base_or_half_size;
                } else if ((ty == 2) || (ty == 3)) {
                    tex_or_rel_pos = (pos - data.base_or_center);
                }

                gl_Position = vec4(((pos - params.offset) / (vec2(params.width, params.height) / 2.) - vec2(1.)), data.z_index, 1.0);
            }
        "
    }
}
mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
            #version 450
            #extension GL_EXT_debug_printf: enable
            layout(set = 0, binding = 1) uniform sampler2D tex;
            layout(location = 0) in vec2 tex_or_rel_pos;
            layout(location = 1) flat in vec2 half_size;
            layout(location = 2) flat in vec4 color;
            layout(location = 3) flat in uint ty;
            layout(location = 4) flat in float inverted;
            layout(location = 5) flat in float border_radius;
            layout(location = 6) flat in float stroke_width;
            layout(location = 7) flat in vec2 clip_min;
            layout(location = 8) flat in vec2 clip_max;
            layout(location = 9) flat in float for_clipping;

            layout(location = 0) out vec4 f_color;

            void main() {
                if (gl_FragCoord.x < clip_min.x || gl_FragCoord.y < clip_min.y
                 || gl_FragCoord.x > clip_max.x || gl_FragCoord.y > clip_max.y) {
                    discard;
                } else {
                    if (ty == 0) { // lyon
                        f_color = color;
                    } else if (ty == 1) { // text
                        float alpha = texture(tex, tex_or_rel_pos).r;
                        f_color = color;
                        f_color.a *= alpha;
                    } else if ((ty == 2) || (ty == 3)) { // rounded rect
                        vec2 abs_pos = abs(tex_or_rel_pos) - half_size;
                        float inner_radius = max(border_radius - stroke_width, 0.0);
                        vec2 outer_pos = abs_pos + border_radius;
                        vec2 inner_pos = abs_pos + stroke_width + inner_radius;
                        float outer = length(max(outer_pos, 0.0)) + min(max(outer_pos.x, outer_pos.y), 0.0);
                        float inner = length(max(inner_pos, 0.0)) + min(max(inner_pos.x, inner_pos.y), 0.0);
                        float from_outer = clamp(0.5 - (outer - border_radius), 0, 1);
                        float from_inner = clamp(0.5 - (inner_radius - inner), 0, 1);
                        float rect_alpha = abs(float(inverted) - from_outer * from_inner);
                        float alpha = color.a * rect_alpha;
                        if (rect_alpha == 0.0 && (for_clipping > 0.0 || from_outer == 0.0)) discard;
                        f_color = vec4(color.rgb, alpha);
                    }
                }
            }
        "
    }
}

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    pos: [f32; 2],
    primitive_index: u32,
}
vulkano::impl_vertex!(Vertex, pos, primitive_index);

#[derive(Debug, Clone, AsStd430)]
pub struct PrimitiveData {
    ty: u32,
    z_index: f32,
    base_or_center: crevice::std430::Vec2,
    color: crevice::std430::Vec4,
    tex_base_or_half_size: crevice::std430::Vec2,
    tex_scale_or_border_radius_and_stroke_width: crevice::std430::Vec2,
    clip_min: crevice::std430::Vec2,
    clip_max: crevice::std430::Vec2,
}

pub struct RenderData {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u32>,
    primitive_data: Vec<<PrimitiveData as AsStd430>::Output>,
}

impl RenderData {
    fn new() -> Self { Self { vertices: vec![], indices: vec![], primitive_data: vec![] } }

    pub fn add_rounded_rect(
        &mut self,
        color: [f32; 4],
        z_index: f32,
        rect: Rect,
        clip: Rect,
        border_radius: f32,
        stroke_width: f32,
        inverted: bool,
        for_clipping: bool,
    ) {
        let primitive_index = self.primitive_data.len() as u32;
        self.primitive_data.push(
            PrimitiveData {
                ty: (if inverted { 3 } else { 2 }) | (if for_clipping { 4 } else { 0 }),
                color: crevice::std430::Vec4 { x: color[0], y: color[1], z: color[2], w: color[3] },
                z_index,
                base_or_center: rect.center().into(),
                tex_base_or_half_size: (rect.size / 2.0).into(),
                tex_scale_or_border_radius_and_stroke_width: crevice::std430::Vec2 {
                    x: border_radius,
                    y: stroke_width,
                },
                clip_min: clip.near_corner().into(),
                clip_max: clip.far_corner().into(),
            }
            .as_std430(),
        );
        let vertex_id = self.vertices.len() as u32;
        self.vertices.push(Vertex { pos: rect.near_corner().into(), primitive_index });
        self.vertices.push(Vertex { pos: [rect.pos.x, rect.pos.y + rect.size.y], primitive_index });
        self.vertices.push(Vertex { pos: [rect.pos.x + rect.size.x, rect.pos.y], primitive_index });
        self.vertices.push(Vertex { pos: rect.far_corner().into(), primitive_index });
        self.push_quad(vertex_id);
    }

    pub fn add_text_quad_data(
        &mut self,
        color: [f32; 4],
        z_index: f32,
        rect: Rect,
        tex_base: Vec2,
        tex_scale: Vec2,
    ) -> u32 {
        let primitive_index = self.primitive_data.len() as u32;
        self.primitive_data.push(
            PrimitiveData {
                ty: 1,
                color: crevice::std430::Vec4 { x: color[0], y: color[1], z: color[2], w: color[3] },
                z_index,
                base_or_center: rect.near_corner().into(),
                tex_base_or_half_size: tex_base.into(),
                tex_scale_or_border_radius_and_stroke_width: tex_scale.into(),
                clip_min: rect.near_corner().into(),
                clip_max: rect.far_corner().into(),
            }
            .as_std430(),
        );
        let vertex_id = self.vertices.len() as u32;
        self.vertices.push(Vertex { pos: rect.near_corner().into(), primitive_index });
        self.vertices.push(Vertex { pos: [rect.pos.x, rect.pos.y + rect.size.y], primitive_index });
        self.vertices.push(Vertex { pos: [rect.pos.x + rect.size.x, rect.pos.y], primitive_index });
        self.vertices.push(Vertex { pos: rect.far_corner().into(), primitive_index });
        vertex_id
    }

    pub fn push_quad(&mut self, vertex_id: u32) {
        self.indices.push(vertex_id + 1);
        self.indices.push(vertex_id);
        self.indices.push(vertex_id + 2);

        self.indices.push(vertex_id + 1);
        self.indices.push(vertex_id + 3);
        self.indices.push(vertex_id + 2);
    }

    pub fn add_lyon_data(
        &mut self,
        color: [f32; 4],
        z_index: f32,
        clip_min: [f32; 2],
        clip_max: [f32; 2],
    ) -> u32 {
        let idx = self.primitive_data.len() as u32;
        self.primitive_data.push(
            PrimitiveData {
                ty: 0,
                color: crevice::std430::Vec4 { x: color[0], y: color[1], z: color[2], w: color[3] },
                z_index,
                base_or_center: Vec2::zero().into(),
                tex_base_or_half_size: Vec2::zero().into(),
                tex_scale_or_border_radius_and_stroke_width: Vec2::zero().into(),
                clip_min: crevice::std430::Vec2 { x: clip_min[0], y: clip_min[1] },
                clip_max: crevice::std430::Vec2 { x: clip_max[0], y: clip_max[1] },
            }
            .as_std430(),
        );
        idx
    }

    pub fn add_lyon_vertex(&mut self, primitive_index: u32, pos: [f32; 2]) -> u32 {
        let idx = self.vertices.len() as u32;
        self.vertices.push(Vertex { pos, primitive_index });
        idx
    }
}

pub struct Renderer {
    queue: Arc<Queue>,
    pipeline: std::sync::Arc<GraphicsPipeline>,
    pub(crate) data: RenderData,
    sampler: Arc<Sampler>,
}
impl Renderer {
    pub fn new(render_pass: Arc<RenderPass>, device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let vs = vertex_shader::Shader::load(device.clone()).unwrap();
        let fs = fragment_shader::Shader::load(device.clone()).unwrap();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input(BuffersDefinition::new().vertex::<Vertex>())
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .blend_collective(AttachmentBlend {
                    enabled: true,
                    color_op: BlendOp::Add,
                    color_source: BlendFactor::SrcAlpha,
                    color_destination: BlendFactor::OneMinusSrcAlpha,
                    alpha_op: BlendOp::Max,
                    alpha_source: BlendFactor::One,
                    alpha_destination: BlendFactor::One,
                    mask_red: true,
                    mask_green: true,
                    mask_blue: true,
                    mask_alpha: true,
                })
                .depth_stencil(DepthStencil {
                    depth_compare: CompareOp::LessOrEqual,
                    ..DepthStencil::simple_depth_test()
                })
                .render_pass(Subpass::from(render_pass, 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let sampler = Sampler::new(
            device,
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


        Self { queue, pipeline, sampler, data: RenderData::new() }
    }
    pub fn render(&mut self, render_object: &PositionedElement) {
        if let PositionedElement {
            element:
                RenderObjectOrSubPass::RenderObject(RenderObject::RoundedRect {
                    stroke_color,
                    fill_color,
                    stroke_width,
                    border_radius,
                    inverted,
                    for_clipping,
                }),
            clipping_rect,
            rect,
            z_index,
        } = render_object
        {
            let border_radius_px = match border_radius {
                Dimension::Paxel(px) => *px,
                Dimension::Fraction(percent) => {
                    (if rect.size.x > rect.size.y { rect.size.y } else { rect.size.x })
                        * percent
                        * 0.5
                }
            };
            if let Some(stroke_color) = stroke_color {
                self.data.add_rounded_rect(
                    stroke_color.into_raw(),
                    1.0 - *z_index as f32 / 65535.0,
                    *rect,
                    clipping_rect.unwrap_or(*rect),
                    border_radius_px,
                    *stroke_width,
                    *inverted,
                    *for_clipping,
                );
            }
            if let Some(fill_color) = fill_color {
                let rect = rect.inset(*stroke_width);
                self.data.add_rounded_rect(
                    fill_color.into_raw(),
                    1.0 - *z_index as f32 / 65535.0,
                    rect,
                    clipping_rect.unwrap_or(rect),
                    (border_radius_px - *stroke_width).max(0.0),
                    rect.size.maximum() / 2.0,
                    *inverted,
                    *for_clipping,
                );
            }
        }
        if let PositionedElement {
            element: RenderObjectOrSubPass::RenderObject(RenderObject::DebugRect),
            clipping_rect,
            rect,
            ..
        } = render_object
        {
            self.data.add_rounded_rect(
                [1.0, 0.0, 0.0, 0.5],
                0.0,
                *rect,
                clipping_rect.unwrap_or(*rect),
                0.0,
                2.0,
                false,
                false,
            );
        }
    }


    pub fn finish(
        &mut self,
        font_texture: Arc<ImmutableImage>,
    ) -> (
        impl GpuFuture,
        impl GpuFuture,
        impl GpuFuture,
        Arc<impl DescriptorSet>,
        Arc<impl BufferAccess>,
        Arc<impl BufferAccess + TypedBufferAccess<Content = [u32]>>,
    ) {
        let layout = self.pipeline.layout().descriptor_set_layouts()[0].clone();

        let texture = ImageView::new(font_texture).unwrap();

        let (primitive_buffer, primitive_fut) = ImmutableBuffer::from_iter(
            self.data.primitive_data.drain(..),
            BufferUsage { storage_buffer: true, ..BufferUsage::none() },
            self.queue.clone(),
        )
        .unwrap();

        let (vertex_buffer, vertex_fut) = ImmutableBuffer::from_iter(
            self.data.vertices.drain(..),
            BufferUsage { vertex_buffer: true, ..BufferUsage::none() },
            self.queue.clone(),
        )
        .unwrap();

        let (index_buffer, index_fut) = ImmutableBuffer::from_iter(
            self.data.indices.drain(..),
            BufferUsage { index_buffer: true, ..BufferUsage::none() },
            self.queue.clone(),
        )
        .unwrap();

        let mut set_builder = PersistentDescriptorSet::start(layout);
        set_builder
            .add_buffer(primitive_buffer)
            .unwrap()
            .add_sampled_image(texture, self.sampler.clone())
            .unwrap();
        let descriptor_set = Arc::new(set_builder.build().unwrap());

        (primitive_fut, vertex_fut, index_fut, descriptor_set, vertex_buffer, index_buffer)
    }

    pub fn render_part<T: DescriptorSet + Send + Sync + 'static>(
        &self,
        buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        descriptor_set: Arc<T>,
        viewport: &Viewport,
        dimensions: &[u32; 2],
        offset: Vec2,
        start: u64,
        end: u64,
    ) {
        let push_constants = vertex_shader::ty::PushConstantData {
            width: dimensions[0],
            height: dimensions[1],
            offset: offset.into(),
        };

        buffer_builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .push_constants(self.pipeline.layout().clone(), 0, push_constants)
            .set_viewport(0, std::iter::once(viewport.clone()))
            .draw_indexed((end - start) as _, 1, start as _, 0, 0)
            .unwrap();
    }
}
