use crate::{
    eval::layout::PositionedRenderObject,
    geom::Rect,
    vulkano_render::renderer::RenderData,
    RenderObject,
    Vec2,
};
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
    pub clipping_rect: Rect,
}
impl Extra {
    fn as_arr(&self) -> [OrderedFloat<f32>; 9] {
        [
            OrderedFloat::from(self.color[0]),
            OrderedFloat::from(self.color[1]),
            OrderedFloat::from(self.color[2]),
            OrderedFloat::from(self.color[3]),
            OrderedFloat::from(self.z),
            OrderedFloat::from(self.clipping_rect.pos.x),
            OrderedFloat::from(self.clipping_rect.pos.y),
            OrderedFloat::from(self.clipping_rect.size.x),
            OrderedFloat::from(self.clipping_rect.size.y),
        ]
    }
}
impl Hash for Extra {
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_arr().hash(state) }
}
impl PartialEq for Extra {
    fn eq(&self, other: &Self) -> bool { self.as_arr() == other.as_arr() }
}

pub struct TextRenderer {
    queue: Arc<Queue>,
    glyph_brush: GlyphBrush<([f32; 4], f32, Rect, Vec2, Vec2), Extra>,
    old_data: Vec<([f32; 4], f32, Rect, Vec2, Vec2)>,
    texture_bytes: Vec<u8>,
    texture: Arc<ImmutableImage>,
    texture_fut: Option<CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>>,
}
impl TextRenderer {
    pub fn new(queue: Arc<Queue>) -> Self {
        let glyph_brush = GlyphBrushBuilder::using_font(FONT.clone()).build();

        let (w, h) = glyph_brush.texture_dimensions();
        let texture_bytes = vec![0u8; (w * h) as usize];

        let (texture, texture_fut) = ImmutableImage::from_iter(
            [0u8].iter().cloned(),
            ImageDimensions::Dim2d { width: 1, height: 1, array_layers: 1 },
            MipmapsCount::One,
            Format::R8Unorm,
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
    pub fn render<'a>(&mut self, render_object: &PositionedRenderObject<'a>) {
        let clipping_rect = if let Some(clipping_rect) = render_object.clipping_rect {
            render_object.rect.clip(clipping_rect)
        } else {
            render_object.rect
        };

        if let RenderObject::Text { text, size, color } = &render_object.render_object {
            self.glyph_brush.queue(
                Section::new()
                    .add_text(Text {
                        text: &*text,
                        scale: PxScale::from(*size),
                        font_id: Default::default(),
                        extra: Extra {
                            color: color.into_linear().into_raw::<[f32; 4]>(),
                            z: 1.0 - render_object.z_index as f32 / 65535.0,
                            clipping_rect,
                        },
                    })
                    .with_screen_position(Into::<(f32, f32)>::into(render_object.rect.pos))
                    .with_bounds(Into::<(f32, f32)>::into(render_object.rect.size)),
            );
        }
    }

    pub fn finish(
        &mut self,
        data: &mut RenderData,
    ) -> (Arc<ImmutableImage>, Option<impl GpuFuture>) {
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
            |vertex_data| {
                let clipping_rect = vertex_data.extra.clipping_rect;
                let original_rect = Rect::from_corners(
                    vertex_data.pixel_coords.min.into(),
                    vertex_data.pixel_coords.max.into(),
                );
                let clipped = original_rect.clip(clipping_rect);

                let original_tex_rect = Rect::from_corners(
                    vertex_data.tex_coords.min.into(),
                    vertex_data.tex_coords.max.into(),
                );
                let clipped_tex_size = (clipped.size / original_rect.size) * original_tex_rect.size;
                let clipped_tex_rect = Rect { pos: original_tex_rect.pos, size: clipped_tex_size };

                (
                    vertex_data.extra.color,
                    vertex_data.extra.z,
                    clipped,
                    clipped_tex_rect.near_corner(),
                    clipped_tex_rect.size / clipped.size,
                )
            },
        ) {
            Ok(BrushAction::Draw(quads)) => {
                for (color, z, rect, tex_base, tex_scale) in &quads {
                    data.add_text_quad(*color, *z, *rect, *tex_base, *tex_scale)
                }
                self.old_data = quads;
            }
            Ok(BrushAction::ReDraw) => {
                for (color, z, rect, tex_base, tex_scale) in &self.old_data {
                    data.add_text_quad(*color, *z, *rect, *tex_base, *tex_scale)
                }
            }
            Err(BrushError::TextureTooSmall { suggested: (w, h) }) => {
                self.texture_bytes = vec![0u8; (w * h) as usize];
                self.glyph_brush.resize_texture(w, h);
            }
        }

        if texture_upload {
            let (texture, texture_fut) = ImmutableImage::from_iter(
                self.texture_bytes.iter().cloned(),
                ImageDimensions::Dim2d { width, height, array_layers: 1 },
                MipmapsCount::One,
                Format::R8Unorm,
                self.queue.clone(),
            )
            .unwrap();
            self.texture = texture;
            self.texture_fut = Some(texture_fut);
        }

        (self.texture.clone(), self.texture_fut.take())
    }
}
