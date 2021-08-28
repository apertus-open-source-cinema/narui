pub mod heart;
pub mod hooks;
pub mod theme;
pub mod util;
pub mod vulkano_render;


// this is a rather evil hack for being able to resolve paths with narui:: as
// crate::
pub(crate) use crate as narui;
pub use narui_macros as macros;

pub use heart::*;
pub use hooks::*;
pub use macros::*;
pub use theme::*;
pub use util::*;
pub use vulkano_render::*;


#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
