// do partial re evaluation of the changed widget tree

use crate::{
    heart::{
        Context,
        EvalObject,
        Key,
        KeyPart,
        LayoutObject,
        LayoutTree,
        RenderObject,
        Widget,
        WidgetLocalContext,
    },
    hooks::{ContextListenable, Listenable},
};
use derivative::Derivative;
use hashbrown::HashSet;
use itertools::Itertools;
use parking_lot::Mutex;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use stretch::{geometry::Size, number::Number, prelude::Style};
use std::mem::{uninitialized, MaybeUninit};

type Deps = HashSet<Key>;
// EvaluatedEvalObject is analog to a EvalObject but not lazy and additionally
// contains the dependencies of Node for allowing partial rebuild.
#[derive(Clone)]
pub struct EvaluatedEvalObject {
    pub children:
        Vec<(KeyPart, EvaluatedEvalObject, Arc<dyn Fn(Context) -> EvalObject + Send + Sync>, Deps)>,
    pub layout_object: Option<LayoutObject>,
}

// The evaluator outputs nothing but rather communicates with the layouter with
// the LayoutTree trait.
pub struct Evaluator<T: LayoutTree> {
    pub context: Context,
    root: EvaluatedEvalObject,
    layout_tree: Arc<Mutex<T>>,
}

impl<T: LayoutTree> Evaluator<T> {
    pub fn new(top_node: EvalObject, mut layout_tree: Arc<Mutex<T>>) -> Self {
        let context = Context::default();
        let mut root = Self::evaluate(Some(top_node), None, context.clone(), layout_tree.clone()).0;
        Evaluator { context, root, layout_tree }
    }
    pub fn update(&mut self) {
        self.root =
            Self::evaluate(None, Some(&self.root), self.context.clone(), self.layout_tree.clone()).0;
        self.context.global.write().update_tree();
    }


    pub fn evaluate(
        eval_obj: Option<EvalObject>,
        last: Option<&EvaluatedEvalObject>,
        context: Context,
        layout_tree: Arc<Mutex<impl LayoutTree>>,
    ) -> (EvaluatedEvalObject, bool) {
        if Self::should_widget_update(last, context.clone()) {
            let (children, layout_object) = match eval_obj {
                None => { (None, None) }
                Some(eval_obj) => { (Some(eval_obj.children), Some(eval_obj.layout_object)) }
            };
            let children = children.unwrap_or_else(|| {
                last.unwrap().children.iter().map(|(a, b, c, ..)| (*a, c.clone())).collect_vec()
            });

            let evaluated_children: Vec<_> = children
                .iter()
                .map(|(key_part, gen)| {
                    let key = context.widget_local.key.with(*key_part);
                    let context = Context {
                        global: context.global.clone(),
                        widget_local: WidgetLocalContext {
                            key,
                            hook_counter: Arc::new(Default::default()),
                            used: Arc::new(Default::default()),
                        },
                    };

                    let last_child = last.and_then(|last| {
                        last.children.iter().find(|child| child.0 == *key_part).map(|x| x.1.clone())
                    });

                    let (evaluated, _) = Self::evaluate(
                        Some(gen(context.clone())),
                        Some(&last_child.unwrap()),
                        context.clone(),
                        layout_tree.clone(),
                    );
                    let used = context.widget_local.used.lock().clone();

                    (*key_part, evaluated, gen.clone(), used)
                })
                .collect();

            layout_tree.lock().set_children(
                Self::get_layout_children(
                    &mut evaluated_children
                        .iter()
                        .map(|(key_part, evaluated, gen, used)| {
                            (*key_part, evaluated.clone())
                        }),
                    context.widget_local.key,
                ).into_iter(),
                context.widget_local.key,
            );

            let evaluated = EvaluatedEvalObject {
                children: evaluated_children,
                layout_object: layout_object.unwrap_or_else(|| last.unwrap().layout_object.clone()),
            };
            (evaluated, true)
        } else {
            let mut last = last.unwrap().clone();

            let mut some_updated = false;
            for (key_part, evaluated, gen, used) in last.children.iter_mut() {
                let key = context.widget_local.key.with(*key_part);
                let context = Context {
                    global: context.global.clone(),
                    widget_local: WidgetLocalContext {
                        key,
                        hook_counter: Arc::new(Default::default()),
                        used: Arc::new(Default::default()),
                    },
                };

                let (new_evaluated, updated) =
                    Self::evaluate(Some(EvalObject { children: vec![(*key_part, gen.clone())], layout_object: None }), Some(&evaluated), context, layout_tree.clone());
                some_updated |= updated;
                *evaluated = new_evaluated;
            }

            if some_updated {
                layout_tree.lock().set_children(
                    Self::get_layout_children(&mut last.children.clone().into_iter().map(|(a, b, ..)| (a, b)), context.widget_local.key).into_iter(),
                    context.widget_local.key,
                );
            }

            (last, false)
        }
    }
    fn get_layout_children(
        children: &mut dyn Iterator<Item = (KeyPart, EvaluatedEvalObject)>,
        base_key: Key,
    ) -> Vec<(Key, LayoutObject)> {
        children
            .flat_map(|(key_part, child)| {
                let key = base_key.with(key_part);
                if let Some(layout_object) = child.layout_object {
                    return vec![(key, layout_object)];
                }
                Self::get_layout_children(&mut child.children.into_iter().map(|(a, b, ..)| (a, b)), key)
            })
            .collect()
    }
    fn should_widget_update(last: Option<&EvaluatedEvalObject>, context: Context) -> bool {
        match last {
            Some(evaluated) => {
                true // todo really check
            }
            None => true,
        }
    }
}
