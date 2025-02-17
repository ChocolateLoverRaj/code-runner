use core::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{sync::Arc, task::Wake};
use futures::pin_mut;

use crate::syscall::syscall_block_until_event;

struct SingleWaker {
    woke_up: Arc<AtomicBool>,
}

impl Wake for SingleWaker {
    fn wake(self: alloc::sync::Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &alloc::sync::Arc<Self>) {
        self.woke_up.store(true, Ordering::Relaxed);
    }
}

/// Execute a single future
pub fn execute_future<T>(future: impl Future<Output = T>) -> T {
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
        if !woke_up.load(Ordering::Relaxed) {
            // FIXME: We could get woken up right here, between the check and the block, resulting in a missed wake-up. This needs a new syscall to simulate `interrupts::disable`, `interrupts::enable_and_hlt`, and `interrupts::enable`.
            syscall_block_until_event();
            woke_up.store(false, Ordering::Relaxed);
        }
    }
}
