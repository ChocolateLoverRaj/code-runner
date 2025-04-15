use core::{
    future::Future,
    sync::atomic::Ordering,
    task::{Context, Poll, Waker},
};

use futures::pin_mut;

use crate::{executor_context::ExecutorContext, syscall::syscall_wait_until_event};

/// Execute a single future
pub fn execute_future<T>(future: impl Future<Output = T>, executor_context: &ExecutorContext) -> T {
    pin_mut!(future);
    // We don't care about getting woken up because we will call poll after receiving any event
    let waker = Waker::noop();
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
