// do partial re evaluation of the changed widget tree

use crate::heart::{Context, EvaluatedWidget, Key, RenderObject, UnevaluatedWidget, Widget, WidgetGen, Fragment, KeyPart, ConcreteWidget, LayoutObject, WidgetLocalContext, EvalObject, LayoutTree};
use derivative::Derivative;
use hashbrown::HashSet;
use itertools::Itertools;
use parking_lot::Mutex;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use stretch::{geometry::Size, number::Number, prelude::Style};
use crate::hooks::{ContextListenable, Listenable};

type Deps = Set<Key>;
// EvaluatedEvalObject is analog to a EvalObject but not lazy and additionally contains
// the dependencies of Node for allowing partial rebuild.
#[derive(Clone)]
pub struct EvaluatedEvalObject {
    pub children: Vec<(KeyPart, EvaluatedEvalObject, Box<dyn Fn(Context) -> EvalObject + Send + Sync>, Deps)>,
    pub layout_object: Option<LayoutObject>,
}

// The evaluator outputs nothing but rather communicates with the layouter with the
// LayoutTree trait.
pub struct Evaluator<T: LayoutTree> {
    pub context: Context,
    root: EvaluatedEvalObject,
    layout_tree: T,
}

impl<T> Evaluator<T> {
    pub fn new(top_node: EvalObject, layout_tree: impl LayoutTree) -> Self {
        let context = Context::default();
        let mut root = Self::evaluate(Some(top_node), &None, context, &mut layout_tree).0;
        let new_ref = &mut root.clone();
        Self::eval_conditional(&mut root, new_ref, context.clone());

        Evaluator { context: context.clone(), root, layout_tree }
    }
    pub fn update(&mut self) {
        self.root = Self::evaluate(None, &Some(self.root), self.context.clone(), &mut self.layout_tree).0;
        self.context.global.write().update_tree();
    }


    pub fn evaluate(eval_obj: Option<EvalObject>, last: &Option<EvaluatedEvalObject>, context: Context, layout_tree: &mut impl LayoutTree) -> (EvaluatedEvalObject, bool) {
        if Self::should_widget_update(&last, context) {
            let evaluated_children: Vec<_> = children.map(|key_part, gen| {
                let key = context.widget_local.key.with(key_part);
                let context = Context { global: context.global.clone(), widget_local: WidgetLocalContext {
                    key,
                    hook_counter: Arc::new(Default::default()),
                    used: Arc::new(Default::default())
                }};

                let last_child = last.and_then(|last| {
                    last.children.iter().find(|child| child.0 == key_part).map(|x| x.1)
                });

                let evaluated = Self::evaluate(gen(context), &last_child, context.clone(), layout_tree);
                let used = context.widget_local.used.lock().clone();

                (key_part, evaluated, gen, used)
            }).collect();

            layout_tree.set_children(Self::get_layout_children(evaluated_children.clone()), key);

            let evaluated = EvaluatedEvalObject {
                children: evaluated_children,
                layout_object: eval_obj.map(|x| x.layout_object).unwrap_or_else(|| last.unwrap().layout_object),
            };
            (evaluated, true)
        } else {
            let mut last = last.unwrap();

            let mut some_updated = false;
            for (key_part, evaluated, gen, used) in last.children.iter_mut() {
                let key = context.widget_local.key.with(*key_part);
                let context = Context { global: context.global.clone(), widget_local: WidgetLocalContext {
                    key,
                    hook_counter: Arc::new(Default::default()),
                    used: Arc::new(Default::default())
                }};

                let (evaluated, updated) = Self::evaluate(gen, &Some(evaluated), context, layout_tree);
                some_updated |= updated;
                *child = evaluated;
            }

            if some_updated {
                layout_tree.set_children(Self::get_layout_children(evaluated_children.clone()), key);
            }

            (last, false)
        }
    }
    fn get_layout_children(children: impl Iterator<Item=EvaluatedEvalObject>) -> Vec<LayoutObject> {
        children.flat_map(|child| {
            if let Some(layout_object) = child.layout_object {
                return vec![layout_object];
            }
            Self::get_layout_children(child.children)
        }).collect()
    }
    fn should_widget_update(last: &Option<EvaluatedEvalObject>, context: Context) -> bool {
        match last {
            Some(evaluated) => {
                true  // todo really check
            }
            None => true
        }
    }
}
