use crate::{
    eval::layout::{PositionedElement, RenderObjectOrSubPass},
    geom::Vec2,
    re_export::lyon::lyon_tessellation::{GeometryBuilderError, VertexId},
    vulkano_render::primitive_renderer::RenderData,
    Color,
    RenderObject,
};
use lyon::{
    lyon_tessellation::{FillTessellator, FillVertex},
    tessellation::{
        FillGeometryBuilder,
        GeometryBuilder,
        StrokeGeometryBuilder,
        StrokeTessellator,
        StrokeVertex,
    },
};
use palette::Pixel;


pub struct ColoredBuffersBuilder<'a> {
    data: &'a mut RenderData,
    pos: Vec2,
    z_index: f32,
    clip_min: [f32; 2],
    clip_max: [f32; 2],
}
impl<'a> ColoredBuffersBuilder<'a> {
    pub fn with_color(&mut self, color: Color) -> NaruiGeometryBuilder {
        NaruiGeometryBuilder::new(
            self.data,
            self.pos,
            self.z_index,
            color.into_raw::<[f32; 4]>(),
            self.clip_min,
            self.clip_max,
        )
    }
}

pub struct NaruiGeometryBuilder<'a> {
    primitive_index: u32,
    position: Vec2,
    data: &'a mut RenderData,
    vertex_offset: u32,
    index_offset: u32,
}

impl<'a> NaruiGeometryBuilder<'a> {
    fn new(
        data: &'a mut RenderData,
        position: Vec2,
        z_index: f32,
        color: [f32; 4],
        clip_min: [f32; 2],
        clip_max: [f32; 2],
    ) -> Self {
        Self {
            primitive_index: data.add_lyon_data(color, z_index, clip_min, clip_max),
            position,
            data,
            vertex_offset: 0,
            index_offset: 0,
        }
    }
}

impl<'a> GeometryBuilder for NaruiGeometryBuilder<'a> {
    fn begin_geometry(&mut self) {
        self.vertex_offset = self.data.vertices.len() as _;
        self.index_offset = self.data.indices.len() as _;
    }

    fn end_geometry(&mut self) {}

    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.data.indices.push(a.0);
        self.data.indices.push(b.0);
        self.data.indices.push(c.0);
    }

    fn abort_geometry(&mut self) {
        self.data.vertices.truncate(self.vertex_offset as usize);
        self.data.indices.truncate(self.index_offset as usize);
    }
}

impl<'a> FillGeometryBuilder for NaruiGeometryBuilder<'a> {
    fn add_fill_vertex(&mut self, vertex: FillVertex) -> Result<VertexId, GeometryBuilderError> {
        Ok(self
            .data
            .add_lyon_vertex(
                self.primitive_index,
                [self.position.x + vertex.position().x, self.position.y + vertex.position().y],
            )
            .into())
    }
}

impl<'a> StrokeGeometryBuilder for NaruiGeometryBuilder<'a> {
    fn add_stroke_vertex(
        &mut self,
        vertex: StrokeVertex,
    ) -> Result<VertexId, GeometryBuilderError> {
        Ok(self
            .data
            .add_lyon_vertex(
                self.primitive_index,
                [self.position.x + vertex.position().x, self.position.y + vertex.position().y],
            )
            .into())
    }
}

pub struct Lyon {
    fill_tessellator: FillTessellator,
    stroke_tessellator: StrokeTessellator,
}
impl Lyon {
    pub fn new() -> Self {
        Self {
            fill_tessellator: FillTessellator::new(),
            stroke_tessellator: StrokeTessellator::new(),
        }
    }
    pub fn render<'a>(&mut self, data: &mut RenderData, render_object: &PositionedElement<'a>) {
        let clipping_rect = if let Some(clipping_rect) = render_object.clipping_rect {
            render_object.rect.clip(clipping_rect)
        } else {
            render_object.rect
        };

        if let RenderObjectOrSubPass::RenderObject(RenderObject::Path { path_gen }) =
            render_object.element
        {
            (path_gen)(
                render_object.rect.size,
                &mut self.fill_tessellator,
                &mut self.stroke_tessellator,
                ColoredBuffersBuilder {
                    data,
                    pos: render_object.rect.pos,
                    z_index: 1.0 - render_object.z_index as f32 / 65535.0,
                    clip_min: clipping_rect.near_corner().into(),
                    clip_max: clipping_rect.far_corner().into(),
                },
            );
        };
    }
}
