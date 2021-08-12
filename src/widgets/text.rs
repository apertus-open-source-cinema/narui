use crate::{style::*, vulkano_render::text_render::FONT, *};
use glyph_brush::{
    ab_glyph::{Font, ScaleFont},
    FontId,
    GlyphPositioner,
    Layout,
    SectionGeometry,
    SectionText,
};
use std::sync::Arc;

// this text primitive is a bit special, because it emits both a layout box and
// a primitive
#[widget(size = 24.0, color = crate::theme::TEXT_WHITE, style = Default::default())]
pub fn text(size: f32, children: String, color: Color, style: Style, context: Context) -> Fragment {
    let children_ = children.clone();
    let measurement_function = move |bounds: Size<Number>| -> Size<f32> {
        let fonts = &[FONT.clone()];
        let sfont = fonts[0].as_scaled(size);
        let map_number = |number| match number {
            Number::Undefined => f32::INFINITY,
            Number::Defined(v) => v,
        };
        let glyphs = Layout::default().calculate_glyphs(
            fonts,
            &SectionGeometry {
                screen_position: (0.0, -sfont.descent()),
                bounds: (map_number(bounds.width), map_number(bounds.height)),
            },
            &[SectionText { text: &children_, scale: size.into(), font_id: FontId(0) }],
        );

        let mut calculated_width: f32 = 0.0;
        let mut calculated_height: f32 = 0.0;
        for glyph in glyphs {
            let h_advance = sfont.h_advance(glyph.glyph.id);
            calculated_width = calculated_width.max(glyph.glyph.position.x + h_advance);
            calculated_height = calculated_height.max(glyph.glyph.position.y);
        }

        Size { width: calculated_width, height: calculated_height }
    };

    let primitive_text = RenderObject::Text { text: children, size, color };

    Fragment {
        key: context.widget_local.key,
        children: vec![],
        layout_object: Some(LayoutObject {
            style,
            measure_function: Some(Arc::new(measurement_function)),
            render_objects: vec![(KeyPart::RenderObject(0), primitive_text)],
        }),
    }
}
