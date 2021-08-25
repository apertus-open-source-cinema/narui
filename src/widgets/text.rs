use crate::{vulkano_render::text_render::FONT, *};
use glyph_brush::{
    ab_glyph::{Font, ScaleFont},
    FontId,
    GlyphPositioner,
    Layout,
    SectionGeometry,
    SectionText,
};
use rutter_layout::{BoxConstraints, ClosureLayout, Size};
use std::sync::Arc;


// this text primitive is a bit special, because it emits both a layout box and
// a primitive
#[widget(size = 24.0, color = crate::theme::TEXT_WHITE)]
pub fn text(
    size: f32,
    children: impl ToString + Clone,
    color: Color,
    context: &mut WidgetContext,
) -> FragmentInner {
    let children_clone = children.to_string().clone();
    let layout_closure = move |constraints: rutter_layout::BoxConstraints| -> rutter_layout::Size {
        let fonts = &[FONT.clone()];
        let sfont = fonts[0].as_scaled(size);
        let glyphs = Layout::default().calculate_glyphs(
            fonts,
            &SectionGeometry {
                screen_position: (0.0, -sfont.descent()),
                bounds: (constraints.max_width, constraints.max_height),
            },
            &[SectionText { text: &children_clone, scale: size.into(), font_id: FontId(0) }],
        );

        let mut calculated_width: f32 = 0.0;
        let mut calculated_height: f32 = 0.0;
        for glyph in glyphs {
            let h_advance = sfont.h_advance(glyph.glyph.id);
            calculated_width = calculated_width.max(glyph.glyph.position.x + h_advance);
            calculated_height = calculated_height.max(glyph.glyph.position.y);
        }
        dbg!(constraints, calculated_width);

        constraints
            .constrain(rutter_layout::Size { width: calculated_width, height: calculated_height })
    };

    FragmentInner::Leaf {
        render_object: RenderObject::Text { text: children.to_string(), size, color },
        layout: Box::new(ClosureLayout { closure: Box::new(layout_closure) }),
    }
}
