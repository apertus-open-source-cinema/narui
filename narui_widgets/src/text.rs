use narui::{vulkano_render::text_render::FONT, *};
use glyph_brush::{
    ab_glyph::{Font, ScaleFont},
    FontId,
    GlyphPositioner,
    Layout,
    SectionGeometry,
    SectionText,
};
use rutter_layout::{BoxConstraints, ClosureLayout, Size, LayoutableChildren};
use std::sync::Arc;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct TextLayout {
    size: f32,
    text: Rc<String>
}

impl rutter_layout::Layout for TextLayout {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> Size {
        assert_eq!(children.len(), 0);
        let fonts = &[FONT.clone()];
        let sfont = fonts[0].as_scaled(self.size);
        let glyphs = Layout::default().calculate_glyphs(
            fonts,
            &SectionGeometry {
                screen_position: (0.0, -sfont.descent()),
                bounds: (constraint.max_width, constraint.max_height),
            },
            &[SectionText { text: &self.text, scale: self.size.into(), font_id: FontId(0) }],
        );

        let mut calculated_width: f32 = 0.0;
        let mut calculated_height: f32 = 0.0;
        for glyph in glyphs {
            let h_advance = sfont.h_advance(glyph.glyph.id);
            calculated_width = calculated_width.max(glyph.glyph.position.x + h_advance);
            calculated_height = calculated_height.max(glyph.glyph.position.y);
        }

        constraint
            .constrain(rutter_layout::Size { width: calculated_width, height: calculated_height })
    }
}

// this text primitive is a bit special, because it emits both a layout box and
// a primitive
#[widget(size = 24.0, color = crate::theme::TEXT_WHITE)]
pub fn text(
    size: f32,
    children: impl ToString + Clone,
    color: Color,
    context: &mut WidgetContext,
) -> FragmentInner {
    let text = Rc::new(children.to_string());

    FragmentInner::Leaf {
        render_object: RenderObject::Text { text: text.clone(), size, color },
        layout: Box::new(TextLayout { size, text }),
    }
}
