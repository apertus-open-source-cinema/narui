use crate::{heart::*, widgets::*};
use narui_derive::widget;

use crate::vulkano_render::text_render::FONT;
use glyph_brush::{
    ab_glyph::{Font, ScaleFont},
    FontId,
    GlyphPositioner,
    Layout,
    SectionGeometry,
    SectionText,
};
use std::any::Any;
use stretch::{
    geometry::Size,
    number::Number,
    style::{Dimension, Style},
};

// this text primitive is a bit special, because it emits both a layout box and
// a primitive
#[widget(size = 24.0, color = crate::theme::TEXT_WHITE, width = Default::default(), height = Default::default())]
pub fn text(
    size: f32,
    children: String,
    color: Color,
    width: Dimension,
    height: Dimension,
) -> WidgetInner {
    let style = Style { size: Size { width, height }, ..Default::default() };
    let children_ = children.clone();
    let measurement_function = move |bounds: Size<Number>| -> Result<Size<f32>, Box<dyn Any>> {
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

        Ok(Size { width: calculated_width, height: calculated_height })
    };

    let primitive_text = RenderObject::Text { text: children, size, color };

    WidgetInner::Leaf {
        style,
        measure_function: Box::new(measurement_function),
        render_objects: vec![primitive_text],
    }
}
