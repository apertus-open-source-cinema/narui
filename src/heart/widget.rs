/* The API to the UI library mimics React ergonomics with hooks. There are two types of widgets:
Primitive and Composed Widgets. Primitive Widgets are the variants of the `PrimitiveWidget` Enum.
Composed Widgets are functions that return either other Composed Widgets or Primitive Widgets.
For layout we create `TreeNodes` with stretch Style attributes.
*/

use crate::heart::*;
use derivative::Derivative;
use lyon::path::Path;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::replace,
    sync::{Arc},
};
use parking_lot::Mutex;
use stretch::{geometry::Size, number::Number, style::Style};

pub type WidgetGen = Arc<dyn Fn() -> WidgetInner + Send + Sync>;

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Unevaluated(UnevaluatedWidget),
    Evaluated(EvaluatedWidget),
    None,
}
impl Widget {
    pub fn is_evaluated(&self) -> bool {
        match self {
            Widget::Evaluated(_) => { true }
            _ => { false }
        }
    }
    pub fn evaluated(&self) -> &EvaluatedWidget {
        match self {
            Widget::Unevaluated(_) => { panic!("Evaluate your widgets before continuing! Widget is {:?}", self) }
            Widget::Evaluated(evaluated) => { evaluated }
            _ => { panic!("None widgets should never occur") }
        }
    }
    pub fn key(&self) -> Key {
        match self {
            Widget::Unevaluated(u) => {u.key.clone()}
            Widget::Evaluated(e) => {e.key.clone()}
            _ => { panic!("None widgets should never occur") }
        }
    }
}
impl Into<Vec<Widget>> for Widget {
    fn into(self) -> Vec<Widget> { vec![self] }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct UnevaluatedWidget {
    pub key: Key,
    #[derivative(Debug = "ignore")]
    pub gen: WidgetGen,
}
impl PartialEq for UnevaluatedWidget {
    fn eq(&self, other: &Self) -> bool { self.key == other.key }
}


#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct EvaluatedWidget {
    pub key: Key,
    pub updated: bool,
    pub inner: Arc<WidgetInner>,
    #[derivative(Debug = "ignore")]
    pub gen: WidgetGen,
}
impl PartialEq for EvaluatedWidget {
    fn eq(&self, other: &Self) -> bool { self.key == other.key }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum WidgetInner {
    Composed {
        widget: Mutex<Widget>,
    },
    Node {
        style: Style,
        children: Mutex<Vec<Widget>>,
        render_objects: Vec<RenderObject>,
    },
    Leaf {
        style: Style,
        #[derivative(Debug = "ignore")]
        measure_function: Box<dyn Fn(Size<Number>) -> Size<f32> + Send + Sync>,
        render_objects: Vec<RenderObject>,
    },
}
impl WidgetInner {
    pub fn render_object(
        render_object: RenderObject,
        children: Vec<Widget>,
        style: Style,
    ) -> WidgetInner {
        WidgetInner::Node { style, children: Mutex::new(children), render_objects: vec![render_object] }
    }
    pub fn layout_block(style: Style, children: Vec<Widget>) -> WidgetInner {
        WidgetInner::Node { style, children: Mutex::new(children), render_objects: vec![] }
    }
}

pub type PathGenInner = Arc<dyn (Fn(Size<f32>) -> Path) + Send + Sync>;
pub type PathGen = StateValue<PathGenInner>;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum RenderObject {
    Path {
        #[derivative(Debug = "ignore")]
        path_gen: PathGen,
        color: Color,
    },
    Text {
        text: String,
        size: f32,
        color: Color,
    },
    Input {
        // this is nothing that gets rendered but instead it gets interpreted by the input handling
        // logic
        hover: StateValue<bool>,
        click: StateValue<bool>,
        position: StateValue<Option<Vec2>>,
    },
}
