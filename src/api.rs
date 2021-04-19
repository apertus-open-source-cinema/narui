use crate::types::Color;
use derivative::Derivative;
use lyon::path::Path;
use std::sync::Arc;
use stretch::{geometry::Size, style::Style};

#[derive(Debug, Clone)]
pub enum Widget {
    LayoutBox(LayoutBox),
    RenderObject(RenderObject),
}
impl Into<Vec<Widget>> for Widget {
    fn into(self) -> Vec<Widget> { vec![self] }
}


#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub style: Style,
    pub children: Vec<Widget>,
}
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderObject {
    Path {
        #[derivative(Debug = "ignore")]
        path_gen: Arc<dyn Fn(Size<f32>) -> Path>,
        color: Color,
    },
    Text {
        text: String,
        size: f32,
        color: Color,
    },
    InputSurface, /* this is nothing that gets rendered but instead it gets interpreted by the
                   * input handling logic */
}
