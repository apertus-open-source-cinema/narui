/* Hooks are part of the heart ergonomics of the narui crate: they help to manage all the state of
the GUI application. For implementing them a few nice hacks are employed:
 */

use super::state::*;
use std::fmt::Debug;

pub fn state<T: 'static + Sync + Send + Debug>(initial: T, context: Context) -> StateValue<T> {
    let state_value: StateValue<T> = StateValue::new(context, "state");
    if !state_value.is_present() {
        state_value.set(initial)
    }
    state_value
}

pub fn rise_detector(to_probe: StateValue<bool>, callback: impl Fn() -> (), context: Context) {
    let last = state(false, context);
    if to_probe.get() && !last.get() {
        callback();
    }
    last.set(to_probe.get());
}

pub fn cache<T: Sync + Send + 'static>(
    val: impl Fn() -> T,
    deps: impl PartialEq + Sync + Send + 'static,
    context: Context,
) -> StateValue<T> {
    let value = StateValue::new(context.clone(), "value");
    let key = StateValue::new(context.clone(), "deps");
    if !value.is_present() {
        value.set(val());
        key.set(deps);
    } else if *key.get_ref() != deps {
        value.set(val());
        key.set(deps);
    }
    value
}

pub fn cache_non_hierarchical<T: Sync + Send + 'static>(
    val: impl Fn() -> T,
    deps: impl PartialEq + Sync + Send + 'static,
    key: &str,
    context: Context,
) -> StateValue<T> {
    let context = Context { key: Key::new(key), tree: context.tree };
    cache(val, deps, context)
}
