use super::{ContextEffect, EffectHandle};
use crate::{ThreadContext, WidgetContext};
use std::{
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
    thread::{spawn, JoinHandle},
};

#[derive(Debug, Clone)]
pub struct ThreadHandle<T> {
    sender: SyncSender<T>,
    join_handle: Option<Arc<JoinHandle<()>>>,
    stop_value: Option<T>,
}

impl<T> ThreadHandle<T> {
    pub fn send(&self, msg: T) -> Result<(), std::sync::mpsc::SendError<T>> {
        self.sender.send(msg)
    }
}

impl<T> Drop for ThreadHandle<T> {
    fn drop(&mut self) {
        self.sender.send(self.stop_value.take().unwrap()).unwrap();
        Arc::try_unwrap(self.join_handle.take().unwrap())
            .unwrap()
            .join()
            .expect("error joining thread");
    }
}

pub trait ContextThread {
    fn thread<T: Send + Sync + Clone + 'static>(
        &mut self,
        callback: impl Fn(ThreadContext, Receiver<T>) + Sync + Send + 'static,
        stop_value: T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<ThreadHandle<T>>;
}

impl<'a> ContextThread for WidgetContext<'a> {
    fn thread<T: Send + Sync + Clone + 'static>(
        &mut self,
        callback: impl Fn(ThreadContext, Receiver<T>) + Sync + Send + 'static,
        stop_value: T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<ThreadHandle<T>> {
        let thread_context = self.thread_context();
        let callback = Arc::new(callback);
        self.effect(
            move |_| {
                let thread_context = thread_context.clone();
                let cloned_callback = callback.clone();
                let (sender, receiver) = sync_channel(8);
                let join_handle = spawn(move || {
                    cloned_callback(thread_context, receiver);
                });

                ThreadHandle {
                    join_handle: Some(Arc::new(join_handle)),
                    sender,
                    stop_value: Some(stop_value.clone()),
                }
            },
            deps,
        )
    }
}
