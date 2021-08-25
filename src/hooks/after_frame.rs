use crate::{CallbackContext, WidgetContext};

pub trait ContextAfterFrame {
    fn after_frame(&mut self, callback: impl for<'a> Fn(&'a CallbackContext) + 'static);
}
impl<'b> ContextAfterFrame for WidgetContext<'b> {
    fn after_frame(&mut self, callback: impl for<'a> Fn(&'a CallbackContext) + 'static) {
        self.after_frame_callbacks.push(Box::new(callback))
    }
}
