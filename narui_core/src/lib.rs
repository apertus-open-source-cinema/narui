mod context;
pub(crate) mod eval;
pub mod hooks;
pub mod re_export;
mod util;
mod vulkano_render;

/// items only for consumption by macros. no user code should depend directly on
/// this
pub mod _macro_api {
    pub use super::{
        context::{
            args::{listen_args, shout_args, ArgRef},
            context::WidgetContext,
            key::{
                internal::{WidgetDebugInfo, WIDGET_INFO},
                KeyPart,
            },
        },
        eval::fragment::{Fragment, FragmentInner, UnevaluatedFragment},
        layout::Transparent,
        re_export::{ctor::ctor, smallvec::smallvec},
        util::{all_eq as all_eq_mod, all_eq::all_eq},
    };
}
pub mod renderer {
    pub use super::vulkano_render::{glyph_brush::FONT, lyon::ColoredBuffersBuilder};
}
pub mod app {
    pub use super::{re_export::winit::window::WindowBuilder, vulkano_render::render::render};
}
pub mod layout {
    pub use rutter_layout::{layouts::*, *};
}
pub use geom::*;
pub use hooks::*;
pub use rutter_layout::{
    layouts::{
        AbsolutePosition,
        Alignment,
        AspectRatio,
        CrossAxisAlignment,
        Dimension::{self, *},
        EdgeInsets,
        FlexFit,
        FractionalSize,
        MainAxisAlignment,
        MainAxisSize,
        StackFit,
    },
    BoxConstraints,
    Offset,
    Size,
};


pub use context::{CallbackContext, Key, ThreadContext, WidgetContext};
pub use eval::fragment::*;
pub use re_export::Color;
pub use util::geom;
