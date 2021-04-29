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
use stretch::{
    geometry::Size,
    number::Number,
    style::{Dimension, Style},
};
/*
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

    WidgetInner::Leaf {
        style,
        measure_function: Box::new(measurement_function),
        render_objects: vec![primitive_text],
    }
}
*/

#[macro_export]
macro_rules! __text_constructor
{
    (@ initial $ ($ args : tt) *) =>
    {
        {
            let size = 24.0 ; let color = crate :: theme :: TEXT_WHITE ; let
            width = Default :: default() ; let height = Default :: default() ;
            __text_constructor !
            (@ parse [size, children, color, width, height, __context,] $
             ($ args) *) ; Widget
            {
                key : (& __context . key) . clone(), inner : LazyVal ::
                new(move ||
                    {
                        fn no_deref_clone<T: Clone>(input: &T) -> T {
                            input.clone()
                        }
                        let args_tuple =
                        (no_deref_clone(&size), no_deref_clone(&children), no_deref_clone(&color),
                         no_deref_clone(&width), no_deref_clone(&height), __context .
                         clone(),) ; let args_state_value = StateValue ::
                        new(__context . clone(), "args") ; if !
                        args_state_value . context . is_present() ||
                        args_state_value . get() != args_tuple
                        { args_state_value . set(args_tuple) ; }
                        args_state_value . context . mark_used() ; let
                        cloned_context = __context . clone() ; let to_return =
                        {
                            text(size, children, color, width, height,
                                 __context,)
                        } ; StateValue ::
                        new(cloned_context . clone(), "used") .
                        set_sneaky(cloned_context . used . clone()) ;
                        to_return
                    })
            }
        }
    } ;
    (@ parse
     [$ size : ident, $ children : ident, $ color : ident, $ width : ident, $
      height : ident, $ __context : ident,] size = $ value : expr, $
     ($ rest : tt) *) =>
    {
        #[allow(non_snake_case, unused)] fn
        return_second_size(_first : f32, second : f32) -> f32 { second } let $
        size = return_second_size($ size, $ value) ; __text_constructor !
        (@ parse
         [$ size, $ children, $ color, $ width, $ height, $ __context,] $
         ($ rest) *) ;
    } ;
    (@ parse
     [$ size : ident, $ children : ident, $ color : ident, $ width : ident, $
      height : ident, $ __context : ident,] children = $ value : expr, $
     ($ rest : tt) *) =>
    {
        #[allow(non_snake_case, unused)] fn
        return_second_children(_first : String, second : String) -> String
        { second } let $ children = $ value ; __text_constructor !
        (@ parse
         [$ size, $ children, $ color, $ width, $ height, $ __context,] $
         ($ rest) *) ;
    } ;
    (@ parse
     [$ size : ident, $ children : ident, $ color : ident, $ width : ident, $
      height : ident, $ __context : ident,] color = $ value : expr, $
     ($ rest : tt) *) =>
    {
        #[allow(non_snake_case, unused)] fn
        return_second_color(_first : Color, second : Color) -> Color
        { second } let $ color = return_second_color($ color, $ value) ;
        __text_constructor !
        (@ parse
         [$ size, $ children, $ color, $ width, $ height, $ __context,] $
         ($ rest) *) ;
    } ;
    (@ parse
     [$ size : ident, $ children : ident, $ color : ident, $ width : ident, $
      height : ident, $ __context : ident,] width = $ value : expr, $
     ($ rest : tt) *) =>
    {
        #[allow(non_snake_case, unused)] fn
        return_second_width(_first : Dimension, second : Dimension) ->
        Dimension { second } let $ width =
        return_second_width($ width, $ value) ; __text_constructor !
        (@ parse
         [$ size, $ children, $ color, $ width, $ height, $ __context,] $
         ($ rest) *) ;
    } ;
    (@ parse
     [$ size : ident, $ children : ident, $ color : ident, $ width : ident, $
      height : ident, $ __context : ident,] height = $ value : expr, $
     ($ rest : tt) *) =>
    {
        #[allow(non_snake_case, unused)] fn
        return_second_height(_first : Dimension, second : Dimension) ->
        Dimension { second } let $ height =
        return_second_height($ height, $ value) ; __text_constructor !
        (@ parse
         [$ size, $ children, $ color, $ width, $ height, $ __context,] $
         ($ rest) *) ;
    } ;
    (@ parse
     [$ size : ident, $ children : ident, $ color : ident, $ width : ident, $
      height : ident, $ __context : ident,] __context = $ value : expr, $
     ($ rest : tt) *) =>
    {
        #[allow(non_snake_case, unused)] fn
        return_second___context(_first : Context, second : Context) -> Context
        { second } let $ __context = $ value ; __text_constructor !
        (@ parse
         [$ size, $ children, $ color, $ width, $ height, $ __context,] $
         ($ rest) *) ;
    } ;
    (@ parse
     [$ size : ident, $ children : ident, $ color : ident, $ width : ident, $
      height : ident, $ __context : ident,]) => { } ;
}
pub use __text_constructor;

pub fn text(
    size: f32,
    children: String,
    color: Color,
    width: Dimension,
    height: Dimension,
    __context: Context,
) -> WidgetInner {
    let style = Style { size: Size { width, height }, ..Default::default() };
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
    WidgetInner::Leaf {
        style,
        measure_function: Box::new(measurement_function),
        render_objects: vec![primitive_text],
    }
}
