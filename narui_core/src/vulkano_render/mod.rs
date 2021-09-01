pub(crate) mod glyph_brush;
mod input_handler;
pub(crate) mod lyon;
pub(crate) mod primitive_renderer;
pub mod raw_render;
pub(crate) mod render;
pub(crate) mod vk_util;


// general idea:
// render as indexed triangle list (strip is not possible because lyon only
// emits a triangle list) the vertex data is just position
// additionally use a shader buffer object for per primitive data,
// per primitive data:
// vec4 fill_color;
// vec4 stroke_color;
// float z_index;
// for text rendering
// vec2 tex_base;
// vec2 tex_scale;
// for rect rendering
// vec2 center;
// vec2 size;
// float border_radius;
// float stroke_width;
// u8 type;
