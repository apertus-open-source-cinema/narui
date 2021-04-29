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
    sync::{Arc, Mutex},
};
use stretch::{geometry::Size, number::Number, style::Style};

pub struct Unevaluated();
pub struct Evaluated();


#[derive(Derivative)]
#[derivative(Debug)]
pub struct Widget {
    pub key: Key,
    #[derivative(Debug = "ignore")]
    pub inner: Box<dyn FnOnce() -> WidgetInner<Widget> + Send + Sync>,
}
impl Clone for Widget {
    fn clone(&self) -> Self {
        Widget {
            key: self.key.clone(),
            inner: Box::new((|| panic!("Clones of Widgets are worthless"))),
        }
    }
}
impl PartialEq for Widget {
    fn eq(&self, other: &Self) -> bool { self.key == other.key }
}
impl Into<Vec<Widget>> for Widget {
    fn into(self) -> Vec<Widget> { vec![self] }
}

#[derive(Clone, Debug)]
pub struct EvaluatedWidget {
    pub key: Key,
    pub updated: bool,
    pub inner: Arc<WidgetInner<EvaluatedWidget>>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum WidgetInner<T> {
    Composed {
        widget: T,
    },
    Node {
        style: Style,
        children: Vec<T>,
        render_objects: Vec<RenderObject>,
    },
    Leaf {
        style: Style,
        #[derivative(Debug = "ignore")]
        measure_function: Box<dyn Fn(Size<Number>) -> Size<f32> + Send + Sync>,
        render_objects: Vec<RenderObject>,
    },
}
impl WidgetInner<Widget> {
    pub fn render_object(
        render_object: RenderObject,
        children: Vec<Widget>,
        style: Style,
    ) -> WidgetInner<Widget> {
        WidgetInner::Node { style, children, render_objects: vec![render_object] }
    }
    pub fn layout_block(style: Style, children: Vec<Widget>) -> WidgetInner<Widget> {
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
