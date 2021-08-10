use crate::heart::Context;

pub trait ContextAfterFrame {
    fn after_frame(&self, callback: impl Fn(Context) + Send + Sync + 'static);
}
impl ContextAfterFrame for Context {
    fn after_frame(&self, callback: impl Fn(Context) + Send + Sync + 'static) {
        self.global.write().after_frame_callbacks.push(Box::new(callback))
    }
}
