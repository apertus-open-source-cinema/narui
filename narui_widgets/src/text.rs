use crate::theme;
use narui::{
    layout::{layout_trait::LayoutableChildren, BoxConstraints, Layout, Size},
    re_export::glyph_brush::{
        ab_glyph::{Font, ScaleFont},
        FontId,
        GlyphPositioner,
        Layout as GbLayout,
        SectionGeometry,
        SectionText,
    },
    renderer::FONT,
    *,
};
use narui_macros::widget;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct TextLayout {
    size: f32,
    text: Rc<String>,
}

impl Layout for TextLayout {
    fn layout(&self, constraint: BoxConstraints, children: LayoutableChildren) -> (Size, u32) {
        assert_eq!(children.len(), 0);
        let fonts = &[FONT.clone()];
        let sfont = fonts[0].as_scaled(self.size);
        let glyphs = GbLayout::default().calculate_glyphs(
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

        (constraint.constrain(Size { width: calculated_width, height: calculated_height }), 1)
    }
}

// this text primitive is a bit special, because it emits both a layout box and
// a primitive
#[widget]
pub fn text(
    #[default(24.0)] size: f32,
    children: impl ToString + Clone,
    #[default(theme::TEXT_WHITE)] color: Color,
    context: &mut WidgetContext,
) -> FragmentInner {
    let text = Rc::new(children.to_string());

    FragmentInner::Leaf {
        render_object: RenderObject::Text {
            key: context.widget_local.key,
            text: text.clone(),
            size,
            color,
        },
        layout: Box::new(TextLayout { size, text }),
    }
}
