/* Hooks are part of the heart ergonomics of the narui crate: they help to manage all the state of
the GUI application. For implementing them a few nice hacks are employed:
 */

use super::state::*;
use std::fmt::Debug;


pub fn state<T>(initial: T, state_value: StateValue<T>) -> StateValue<T>
where
    T: 'static + Sync + Send + Debug,
{
    if !state_value.is_present() {
        state_value.set(initial)
    }
    state_value
}
