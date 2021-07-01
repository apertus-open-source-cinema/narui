#![feature(box_syntax)]

pub mod heart;
pub mod theme;
pub mod vulkano_render;
pub mod widgets;
pub mod hooks;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;


#[macro_export]
macro_rules! normal_macro {
    ($ context : tt) => {
        let bla = move | __context : Context | {$context} ;
    } ;
}