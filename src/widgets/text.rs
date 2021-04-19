use crate::{
    api::{RenderObject, Widget},
    hooks::{state, Context},
    types::Color,
    widgets::*,
};
use narui_derive::{hook, rsx, widget};

use crate::api::LayoutBox;
use glyph_brush::{
    ab_glyph::{Font, FontRef, ScaleFont},
    FontId,
    GlyphBrushBuilder,
    GlyphPositioner,
    Layout,
    SectionGeometry,
    SectionText,
};
use notosans::REGULAR_TTF as FONT;
use stretch::{
    geometry::Size,
    style::{Dimension, Style},
};

// this text primitive is a bit special, because it emits both a layout box and
// a primitive
#[widget(size = 24.0, color = Color::black(), width = f32::INFINITY, height = f32::INFINITY)]
pub fn text(size: f32, children: String, color: Color, width: f32, height: f32) -> Widget {
    let droid_sans = FontRef::try_from_slice(FONT).unwrap();
    let fonts = &[droid_sans];
    let glyphs = Layout::default().calculate_glyphs(
        fonts,
        &SectionGeometry { screen_position: (0.0, 0.0), bounds: (width, f32::INFINITY) },
        &[SectionText { text: &children, scale: size.into(), font_id: FontId(0) }],
    );

    let mut calculated_width: f32 = 0.0;
    let mut calculated_height: f32 = 0.0;
    for glyph in glyphs {
        let sfont = fonts[glyph.font_id].as_scaled(glyph.glyph.scale);
        let h_advance = sfont.h_advance(glyph.glyph.id);
        let h_side_bearing = sfont.h_side_bearing(glyph.glyph.id);
        let height = sfont.height();

        calculated_width = calculated_height.max(glyph.glyph.position.x + h_advance);
        calculated_height = calculated_height.max(height);
    }
    let style = Style {
        size: Size {
            width: Dimension::Points(if width == f32::INFINITY { calculated_width } else { width }),
            height: Dimension::Points(
                if height == f32::INFINITY { calculated_height } else { height },
            ),
        },
        ..Default::default()
    };

    let primitive_text = Widget::RenderObject(RenderObject::Text { text: children, size, color });
    let debug_box = rsx! {
        <rounded_rect color=Color::apertus_orange() border_radius=0.0 />
    };

    Widget::LayoutBox(LayoutBox { style, children: vec![primitive_text /* debug_box */] })
}
