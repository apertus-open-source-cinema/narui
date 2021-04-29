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
    mem::replace,
    sync::{Arc, Mutex},
};
use stretch::{geometry::Size, number::Number, style::Style};
use std::hash::{Hash, Hasher};

#[derive(Derivative)]
#[derivative(Debug)]
pub enum LazyValInner<T> {
    Unevaluated(#[derivative(Debug = "ignore")] Box<dyn FnOnce() -> T>),
    Evaluated(Arc<T>),
    Unavailable,
}
#[derive(Debug)]
pub struct LazyVal<T>(Mutex<LazyValInner<T>>);
impl<T> LazyVal<T> {
    pub fn new(gen: impl FnOnce() -> T + 'static) -> Self {
        LazyVal(Mutex::new(LazyValInner::Unevaluated(Box::new(gen))))
    }
    pub fn get(&self) -> Arc<T> {
        let mut lock = self.0.lock().unwrap();
        let val = replace(&mut *lock, LazyValInner::Unavailable);
        match val {
            LazyValInner::Unevaluated(f) => {
                let v = Arc::new(f());
                *lock = LazyValInner::Evaluated(v.clone());
                v
            }
            LazyValInner::Evaluated(v) => v.clone(),
            _ => unimplemented!(),
        }
    }
}
impl<T> Clone for LazyVal<T> {
    fn clone(&self) -> Self { LazyVal(Mutex::new(LazyValInner::Evaluated(self.get()))) }
}

#[derive(Debug, Clone)]
pub struct Widget {
    pub key: Key,
    pub inner: LazyVal<WidgetInner>,
}
impl Hash for Widget {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}
impl Into<Vec<Widget>> for Widget {
    fn into(self) -> Vec<Widget> { vec![self] }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum WidgetInner {
    Composed {
        widget: Widget,
    },
    Node {
        style: Style,
        children: Vec<Widget>,
        render_objects: Vec<RenderObject>,
    },
    Leaf {
        style: Style,
        #[derivative(Debug = "ignore")]
        measure_function: Box<dyn Fn(Size<Number>) -> Size<f32>>,
        render_objects: Vec<RenderObject>,
    },
}
impl WidgetInner {
    pub fn render_object(render_object: RenderObject, children: Vec<Widget>, style: Style) -> Self {
        WidgetInner::Node { style, children, render_objects: vec![render_object] }
    }
    pub fn layout_block(style: Style, children: Vec<Widget>) -> Self {
        WidgetInner::Node { style, children, render_objects: vec![] }
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
