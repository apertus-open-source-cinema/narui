use crate::{
    eval::layout::{Physical, PhysicalPositionedElement, RenderObjectOrSubPass, ScaleFactor},
    geom::Rect,
    vulkano_render::primitive_renderer::RenderData,
    Key,
    RenderObject,
    Vec2,
};
use glyph_brush::{
    ab_glyph::{FontArc, PxScale},
    BrushAction,
    BrushError,
    GlyphBrushBuilder,
    Section,
    Text,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use ordered_float::OrderedFloat;
use palette::Pixel;
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use vulkano::{
    command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
    device::Queue,
    format::Format,
    image::{ImageDimensions, ImmutableImage, MipmapsCount},
    sync::{GpuFuture, NowFuture},
};

lazy_static! {
    pub static ref FONT: FontArc = FontArc::try_from_slice(notosans::REGULAR_TTF).unwrap();
}

#[derive(Debug, Copy, Clone)]
struct Extra {
    pub color: [f32; 4],
    pub z: f32,
    pub clipping_rect: Physical<Rect>,
    pub key: Key,
}
impl Extra {
    fn as_arr(&self) -> [OrderedFloat<f32>; 9] {
        let clipping_rect = self.clipping_rect.inner();
        [
            OrderedFloat::from(self.color[0]),
            OrderedFloat::from(self.color[1]),
            OrderedFloat::from(self.color[2]),
            OrderedFloat::from(self.color[3]),
            OrderedFloat::from(self.z),
            OrderedFloat::from(clipping_rect.pos.x),
            OrderedFloat::from(clipping_rect.pos.y),
            OrderedFloat::from(clipping_rect.size.x),
            OrderedFloat::from(clipping_rect.size.y),
        ]
    }
}
impl Hash for Extra {
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_arr().hash(state) }
}
impl PartialEq for Extra {
    fn eq(&self, other: &Self) -> bool { self.as_arr() == other.as_arr() }
}

type VertexData = (Key, [f32; 4], f32, Physical<Rect>, Vec2, Vec2);

pub struct GlyphBrush {
    queue: Arc<Queue>,
    glyph_brush: glyph_brush::GlyphBrush<VertexData, Extra>,
    old_data: Vec<VertexData>,
    texture_bytes: Vec<u8>,
    texture: Arc<ImmutableImage>,
    texture_fut: Option<CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>>,
}

#[derive(Default)]
pub struct GlyphBrushState {
    data: HashMap<Key, tinyset::Set64<u32>>,
}

impl GlyphBrushState {
    pub fn add(&mut self, key: Key, idx: u32) { self.data.entry(key).or_default().insert(idx); }

    pub fn lookup(&'_ mut self, key: Key) -> impl Iterator<Item = u32> + '_ {
        self.data.entry(key).or_default().iter()
    }
}

impl GlyphBrush {
    pub fn new(queue: Arc<Queue>) -> Self {
        let glyph_brush = GlyphBrushBuilder::using_font(FONT.clone()).build();

        let (w, h) = glyph_brush.texture_dimensions();
        let texture_bytes = vec![0u8; (w * h) as usize];

        let (texture, texture_fut) = ImmutableImage::from_iter(
            [0u8].iter().cloned(),
            ImageDimensions::Dim2d { width: 1, height: 1, array_layers: 1 },
            MipmapsCount::One,
            Format::R8_UNORM,
            queue.clone(),
        )
        .unwrap();

        Self {
            queue,
            glyph_brush,
            old_data: vec![],
            texture_bytes,
            texture,
            texture_fut: Some(texture_fut),
        }
    }

    pub fn prerender<'a>(
        &mut self,
        render_object: &PhysicalPositionedElement<'a>,
        scale_factor: ScaleFactor,
    ) {
        let clipping_rect = if let Some(clipping_rect) = render_object.clipping_rect {
            render_object
                .rect
                .map(|rect| clipping_rect.map(|clipping_rect| rect.clip(clipping_rect)))
                .into()
        } else {
            render_object.rect
        };

        if let RenderObjectOrSubPass::RenderObject(RenderObject::Text { key, text, size, color }) =
            &render_object.element
        {
            self.glyph_brush.queue(
                Section::new()
                    .add_text(Text {
                        text: &*text,
                        scale: PxScale::from(*size * scale_factor.0),
                        font_id: Default::default(),
                        extra: Extra {
                            color: color.into_linear().into_raw::<[f32; 4]>(),
                            z: 1.0 - render_object.z_index as f32 / 65535.0,
                            clipping_rect,
                            key: *key,
                        },
                    })
                    .with_screen_position(render_object.rect.unwrap_physical().pos)
                    .with_bounds(render_object.rect.unwrap_physical().size),
            );
        }
    }

    pub fn finish(
        &mut self,
        data: &mut RenderData,
    ) -> (GlyphBrushState, Arc<ImmutableImage>, Option<impl GpuFuture>) {
        let (width, height) = self.glyph_brush.texture_dimensions();
        let texture_bytes = &mut self.texture_bytes;
        let mut texture_upload = false;
        let mut state = GlyphBrushState::default();
        match (
            std::mem::take(&mut self.old_data),
            self.glyph_brush.process_queued(
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
                |vertex_data| {
                    let clipping_rect = vertex_data.extra.clipping_rect;

                    let original_rect = Rect::from_corners(
                        vertex_data.pixel_coords.min.into(),
                        vertex_data.pixel_coords.max.into(),
                    );
                    let clipped =
                        clipping_rect.map(|clipping_rect| original_rect.clip(clipping_rect));

                    let original_tex_rect = Rect::from_corners(
                        vertex_data.tex_coords.min.into(),
                        vertex_data.tex_coords.max.into(),
                    );
                    let clipped_tex_size = (clipped.unwrap_physical().size / original_rect.size)
                        * original_tex_rect.size;
                    let clipped_tex_rect =
                        Rect { pos: original_tex_rect.pos, size: clipped_tex_size };

                    (
                        vertex_data.extra.key,
                        vertex_data.extra.color,
                        vertex_data.extra.z,
                        clipped,
                        clipped_tex_rect.near_corner(),
                        clipped_tex_rect.size / clipped.unwrap_physical().size,
                    )
                },
            ),
        ) {
            (_, Ok(BrushAction::Draw(quads))) | (quads, Ok(BrushAction::ReDraw)) => {
                for (key, color, z, rect, tex_base, tex_scale) in &quads {
                    let vertex_idx =
                        data.add_text_quad_data(*color, *z, *rect, *tex_base, *tex_scale);
                    state.add(*key, vertex_idx);
                }
                self.old_data = quads;
            }
            (quads, Err(BrushError::TextureTooSmall { suggested: (w, h) })) => {
                self.texture_bytes = vec![0u8; (w * h) as usize];
                self.glyph_brush.resize_texture(w, h);
                self.old_data = quads;
            }
        }

        if texture_upload {
            let (texture, texture_fut) = ImmutableImage::from_iter(
                self.texture_bytes.iter().cloned(),
                ImageDimensions::Dim2d { width, height, array_layers: 1 },
                MipmapsCount::One,
                Format::R8_UNORM,
                self.queue.clone(),
            )
            .unwrap();
            self.texture = texture;
            self.texture_fut = Some(texture_fut);
        }

        (state, self.texture.clone(), self.texture_fut.take())
    }

    pub fn render<'a>(
        &mut self,
        render_object: &PhysicalPositionedElement<'a>,
        data: &mut RenderData,
        state: &mut GlyphBrushState,
    ) {
        if let RenderObjectOrSubPass::RenderObject(RenderObject::Text { key, .. }) =
            &render_object.element
        {
            for idx in state.lookup(*key) {
                data.push_quad(idx);
            }
        }
    }
}
