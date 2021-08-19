#[allow(clippy::too_many_arguments)]
mod controls;
pub use controls::*;

#[path = "fragment.rs"]
mod fragment_widget;
pub use fragment_widget::*;

mod layout;
pub use layout::*;

#[path = "input.rs"]
mod input_widget;
pub use input_widget::*;

#[path = "rect.rs"]
mod rect_widget;
pub use rect_widget::*;

#[path = "text.rs"]
mod text_widget;
pub use text_widget::*;
