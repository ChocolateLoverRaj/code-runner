use core::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{sync::Arc, task::Wake};
use futures::pin_mut;

use crate::{executor_context::ExecutorContext, syscall::syscall_wait_until_event};

struct SingleWaker {
    woke_up: Arc<AtomicBool>,
}

impl Wake for SingleWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.woke_up.store(true, Ordering::Relaxed);
    }
}

/// Execute a single future
pub fn execute_future<T>(future: impl Future<Output = T>, executor_context: &ExecutorContext) -> T {
    pin_mut!(future);
    let woke_up = Arc::new(AtomicBool::new(false));
    let waker = Waker::from(Arc::new(SingleWaker {
        woke_up: woke_up.clone(),
    }));
    let mut context = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => break value,
            Poll::Pending => {}
        }
        syscall_wait_until_event();
        executor_context
            .keyboard_event_received
            .store(true, Ordering::Release);
        executor_context.keyboard_waker.wake();
    }
}
