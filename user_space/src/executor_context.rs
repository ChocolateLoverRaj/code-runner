use core::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

#[derive(Debug, Default)]
pub struct ExecutorContext {
    pub keyboard_waker: AtomicWaker,
    pub keyboard_event_received: AtomicBool,
}
