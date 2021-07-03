pub mod heart;
pub mod hooks;
pub mod theme;
pub mod vulkano_render;
pub mod widgets;

pub use narui_macros as macros;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
