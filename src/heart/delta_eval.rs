// do partial re evaluation of the changed widget tree

use crate::heart::{Context, EvaluatedWidget, Key, RenderObject, StateValue, Widget, WidgetInner};
use derivative::Derivative;
use hashbrown::HashSet;
use itertools::Itertools;
use std::sync::{Arc, Mutex};
use stretch::{geometry::Size, number::Number, prelude::Style};

pub struct Evaluator {
    context: Context,
    root: EvaluatedWidget,
    root_unevaluated: Box<dyn Fn(Context) -> Widget + 'static>,
}

impl Evaluator {
    pub fn new(top_node: impl Fn(Context) -> Widget + 'static) -> Self {
        let context = Context::default();

        Evaluator {
            context: context.clone(),
            root: Self::eval_unconditional(top_node(context.clone())),
            root_unevaluated: Box::new(top_node),
        }
    }

    fn eval_unconditional(input: Widget) -> EvaluatedWidget {
        let inner = match (input.inner)() {
            WidgetInner::Composed { widget } => {
                WidgetInner::Composed { widget: Self::eval_unconditional(widget) }
            }
            WidgetInner::Node { style, children, render_objects } => WidgetInner::Node {
                children: children
                    .into_iter()
                    .map(|child| Self::eval_unconditional(child))
                    .collect_vec(),
                render_objects,
                style,
            },
            WidgetInner::Leaf { style, measure_function, render_objects } => {
                WidgetInner::Leaf { style, measure_function, render_objects }
            }
        };

        EvaluatedWidget { key: input.key, inner: Arc::new(inner), updated: true }
    }

    pub fn update(&mut self) -> EvaluatedWidget {
        let touched = self.context.finish_touched().lock().unwrap().clone();
        dbg!(&touched);
        self.root = self.eval_conditional(
            (self.root_unevaluated)(self.context.clone()),
            Some(self.root.clone()),
            &touched,
        );
        self.root.clone()
    }

    fn eval_conditional(
        &mut self,
        top: Widget,
        old_widget: Option<EvaluatedWidget>,
        touched: &HashSet<Key>,
    ) -> EvaluatedWidget {
        let should_re_eval =
            old_widget.is_some() && self.should_widget_update(top.key.clone(), touched);

        if should_re_eval {
            let inner = match (top.inner)() {
                WidgetInner::Composed { widget } => {
                    let old_widget =
                        if let WidgetInner::Composed { widget } = &*old_widget.unwrap().inner {
                            Some(widget.clone())
                        } else {
                            None
                        };
                    WidgetInner::Composed {
                        widget: self.eval_conditional(widget, old_widget, touched),
                    }
                }
                WidgetInner::Node { style, children, render_objects } => {
                    let old_widgets =
                        if let WidgetInner::Node { children, .. } = &*old_widget.unwrap().inner {
                            children.clone()
                        } else {
                            vec![]
                        };

                    WidgetInner::Node {
                        children: children
                            .into_iter()
                            .enumerate()
                            .map(|(i, child)| {
                                self.eval_conditional(child, old_widgets.get(i).cloned(), touched)
                            })
                            .collect_vec(),
                        render_objects,
                        style,
                    }
                }
                WidgetInner::Leaf { style, measure_function, render_objects } => {
                    WidgetInner::Leaf { style, measure_function, render_objects }
                }
            };
            EvaluatedWidget { key: top.key, inner: Arc::new(inner), updated: true }
        } else {
            old_widget.unwrap()
        }
    }

    fn should_widget_update(&self, key: Key, touched: &HashSet<Key>) -> bool {
        let used: StateValue<Arc<Mutex<HashSet<Key>>>> =
            StateValue::new(self.context.with_key(key), "used");
        let should_update = !used.get_ref().lock().unwrap().is_disjoint(touched);
        should_update
    }
}
