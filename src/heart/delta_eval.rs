// do partial re evaluation of the changed widget tree

use crate::heart::{Context, EvaluatedWidget, Key, RenderObject, StateValue, UnevaluatedWidget, WidgetInner, WidgetGen, Widget};
use derivative::Derivative;
use hashbrown::HashSet;
use itertools::Itertools;
use std::sync::{Arc};
use parking_lot::Mutex;
use stretch::{geometry::Size, number::Number, prelude::Style};
use std::ops::{Deref, DerefMut};

pub struct Evaluator {
    context: Context,
    root: Widget,
}

impl Evaluator {
    pub fn new(top_node: impl Fn(Context) -> Widget + 'static) -> Self {
        let context = Context::default();

        let mut root = top_node(context.clone());
        let new_ref = &mut root.clone();
        Self::eval_conditional(&mut root, new_ref, context.clone());

        Evaluator { context: context.clone(), root, }
    }

    pub fn update(&mut self) -> Widget {
        let new_ref = &mut self.root.clone();
        Self::eval_conditional(&mut self.root, new_ref, self.context.clone());
        self.context.finish_touched();

        dbg!(&self.root);

        self.root.clone()
    }
    fn should_widget_update(key: Key, context: Context) -> bool {
        let used: StateValue<Arc<Mutex<HashSet<Key>>>> =
            StateValue::new(context.with_key(key), "used");
        //dbg!(&*context.touched.lock());
        let should_update = !used.get_ref().lock().is_disjoint(&*context.touched.lock());
        should_update
    }
    fn eval_conditional(
        mut old: impl DerefMut<Target=Widget>,
        new: impl DerefMut<Target=Widget>,
        context: Context,
    ) {
        let should_re_eval = !old.is_evaluated() || Self::should_widget_update(new.key(), context.clone());
        let gen = match (&*new) {
            Widget::Unevaluated(u) => u.gen.clone(),
            Widget::Evaluated(e) => e.gen.clone(),
            _ => panic!("New Widget is None which should not happen"),
        };
        let newly_created =
            if should_re_eval {
                let inner = gen();
                EvaluatedWidget { key: new.key().clone(), inner: Arc::new(inner), updated: true, gen }
            } else {
                let inner = match (&*old) {
                    Widget::Evaluated(e) => e.inner.clone(),
                    _ => panic!("can only not re_eval if there is already a eval version"),
                };
                EvaluatedWidget { key: new.key().clone(), inner, updated: false, gen }
            };

        let inner_old = match (&*old) {
            Widget::Evaluated(e) => Some(e.inner.clone()),
            _ => None,
        };

        match &*newly_created.inner {
            WidgetInner::Composed { widget } => {
                let mut widget = widget.lock();

                let mut fallback = true;
                if let Some(inner_old) = inner_old {
                    if let WidgetInner::Composed { widget: old_widget } = &*inner_old {
                        fallback = false;

                        if !Arc::ptr_eq(&inner_old, &newly_created.inner) {
                            let mut old_widget = old_widget.lock();
                            Self::eval_conditional(&mut *old_widget, &mut widget.clone(), context.clone());
                            *widget = old_widget.clone();
                        } else {
                            let cloned_widget = &mut widget.clone();
                            Self::eval_conditional(&mut *widget, cloned_widget, context.clone());
                        };
                    }
                }

                if fallback {
                    let new_ref = &mut widget.clone();
                    Self::eval_conditional(widget, new_ref, context.clone());
                }
            }

            WidgetInner::Node { children, .. } => {
                let mut children = children.lock();

                let mut fallback = true;
                if let Some(inner_old) = inner_old {
                    if let WidgetInner::Node { children: old_children, .. } = &*inner_old {
                        fallback = false;

                        if !Arc::ptr_eq(&inner_old, &newly_created.inner) {
                            let mut old_children = old_children.lock();

                            (&mut *old_children).resize(children.len(), Widget::None);
                            for (old, new) in (&mut *children).into_iter().zip(&mut *old_children) {
                                Self::eval_conditional(old, new, context.clone());
                            }

                            *children = old_children.clone()
                        } else {
                            for old in &mut *children {
                                let mut new = old.clone();
                                Self::eval_conditional(old, &mut new, context.clone());
                            }
                        };
                    }
                }

                if fallback {
                    for child in &mut *children {
                        let new_ref = &mut child.clone();
                        Self::eval_conditional(child, new_ref, context.clone());
                    }
                }
            },

            WidgetInner::Leaf { .. } => {}
        };

        *old = Widget::Evaluated(newly_created);
    }
}
