use crate::{
    eval::layout::{
        Physical,
        PhysicalPositionedElement,
        RenderObjectOrSubPass,
        ScaleFactor,
        ToPhysical,
    },
    geom::{Rect, Vec2},
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
    clipping_rect: Physical<Rect>,
    scale_factor: ScaleFactor,
}
impl<'a> ColoredBuffersBuilder<'a> {
    pub fn with_color(&mut self, color: Color) -> NaruiGeometryBuilder {
        NaruiGeometryBuilder::new(
            self.data,
            self.pos,
            self.z_index,
            color.into_raw::<[f32; 4]>(),
            self.clipping_rect,
            self.scale_factor,
        )
    }
}

pub struct NaruiGeometryBuilder<'a> {
    primitive_index: u32,
    position: Vec2,
    data: &'a mut RenderData,
    vertex_offset: u32,
    index_offset: u32,
    scale_factor: ScaleFactor,
}

impl<'a> NaruiGeometryBuilder<'a> {
    fn new(
        data: &'a mut RenderData,
        position: Vec2,
        z_index: f32,
        color: [f32; 4],
        clipping_rect: Physical<Rect>,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            primitive_index: data.add_lyon_data(
                color,
                z_index,
                clipping_rect.map(|r| r.near_corner()),
                clipping_rect.map(|r| r.far_corner()),
            ),
            position,
            data,
            vertex_offset: 0,
            index_offset: 0,
            scale_factor,
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
                (self.position + Vec2::from(vertex.position())).to_physical(self.scale_factor),
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
                (self.position + Vec2::from(vertex.position())).to_physical(self.scale_factor),
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
    pub fn render<'a>(
        &mut self,
        data: &mut RenderData,
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

        if let RenderObjectOrSubPass::RenderObject(RenderObject::Path { path_gen }) =
            render_object.element
        {
            (path_gen)(
                render_object.rect.to_logical(scale_factor).size,
                &mut self.fill_tessellator,
                &mut self.stroke_tessellator,
                ColoredBuffersBuilder {
                    data,
                    pos: render_object.rect.map(|r| r.pos).to_logical(scale_factor),
                    z_index: 1.0 - render_object.z_index as f32 / 65535.0,
                    clipping_rect,
                    scale_factor,
                },
            );
        };
    }
}
