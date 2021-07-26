use crate::{Context, ContextEffect, EffectHandle, Key};
use std::{
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
    thread::{spawn, JoinHandle},
};

pub struct ThreadHandle<T> {
    sender: SyncSender<T>,
    join_handle: Option<JoinHandle<()>>,
    stop_value: Option<T>,
}
impl<T> Drop for ThreadHandle<T> {
    fn drop(&mut self) {
        self.sender.send(self.stop_value.take().unwrap()).unwrap();
        self.join_handle.take().unwrap().join().expect("error joining thread");
    }
}


pub trait ContextThread {
    fn thread_key<T: Send + Sync + Clone + 'static>(
        &self,
        key: Key,
        callback: impl Fn(Context, Receiver<T>) + Sync + Send + 'static,
        stop_value: T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<ThreadHandle<T>>;
    fn thread<T: Send + Sync + Clone + 'static>(
        &self,
        callback: impl Fn(Context, Receiver<T>) + Sync + Send + 'static,
        stop_value: T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<ThreadHandle<T>>;
}
impl ContextThread for Context {
    fn thread_key<T: Send + Sync + Clone + 'static>(
        &self,
        key: Key,
        callback: impl Fn(Context, Receiver<T>) + Sync + Send + 'static,
        stop_value: T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<ThreadHandle<T>> {
        let cloned_context = self.clone();
        let callback = Arc::new(callback);
        self.effect_key(
            key,
            move || {
                let cloned_context = cloned_context.clone();
                let cloned_callback = callback.clone();
                let (sender, receiver) = sync_channel(8);
                let join_handle = spawn(move || {
                    cloned_callback(cloned_context, receiver);
                });

                ThreadHandle {
                    join_handle: Some(join_handle),
                    sender,
                    stop_value: Some(stop_value.clone()),
                }
            },
            deps,
        )
    }
    fn thread<T: Send + Sync + Clone + 'static>(
        &self,
        callback: impl Fn(Context, Receiver<T>) + Sync + Send + 'static,
        stop_value: T,
        deps: impl PartialEq + Send + Sync + 'static,
    ) -> EffectHandle<ThreadHandle<T>> {
        self.thread_key(self.key_for_hook(), callback, stop_value, deps)
    }
}
