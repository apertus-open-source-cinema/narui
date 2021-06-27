/* Hooks are part of the heart ergonomics of the narui crate: they help to manage all the state of
the GUI application. For implementing them a few nice hacks are employed:
 */

use super::state::*;
use std::fmt::Debug;

pub fn state<T: 'static + Sync + Send + Debug + Clone>(initial: T, context: Context) -> StateValue<T> {
    let state_value: StateValue<T> = StateValue::new(context, "state");
    if !state_value.context.is_present() {
        state_value.set_now(initial.clone());
        state_value.set(initial);
    }
    state_value
}

pub fn rise_detector(to_probe: StateValue<bool>, callback: impl Fn() -> (), context: Context) {
    let last = state(false, context.clone());
    context.mark_used(&to_probe.context.key);
    context.mark_used(&last.context.key);
    if to_probe.get_sneaky() && !last.get_sneaky() {
        callback();
    }
    if to_probe.get_sneaky() != last.get_sneaky() {
        last.set(to_probe.get_sneaky());
    }
}

pub fn on(callback: impl Fn(), context: Context) -> Option<StateValue<bool>> {
    let state_value = state(false, context.clone());
    rise_detector(state_value.clone(), callback, context.clone());
    Some(state_value)
}

pub fn effect<T: Sync + Send + 'static + Clone>(
    val: impl Fn() -> T,
    deps: impl PartialEq + Debug + Sync + Send + Clone + 'static,
    context: Context,
) -> StateValue<T> {
    let value = StateValue::new(context.clone(), "value");
    let key = StateValue::new(context.clone(), "deps");
    context.mark_used(&key.context.key);
    if !value.context.is_present() {
        let v = val();
        value.set_now(v.clone());
        value.set(v);
        key.set_now(deps.clone());
        key.set_now(deps);
    } else if *key.get_ref_sneaky() != deps {
        value.set(val());
        key.set(deps);
    }
    value
}

pub fn effect_flat<T: Sync + Send + 'static + Clone>(
    val: impl Fn() -> T,
    deps: impl PartialEq + Debug + Sync + Send + 'static + Clone,
    flat_key: &str,
    context: Context,
) -> StateValue<T> {
    let context = Context {
        key: Key::sideband(flat_key.to_string()),
        tree: context.tree,
        tree_next: context.tree_next,
        touched: context.touched,
        used: context.used,
    };
    effect(val, deps, context)
}
