use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{boxed::Box, sync::Arc, task::Wake};
use x86_64::instructions::interrupts;

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
pub fn execute_future<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
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
        interrupts::disable();
        if !woke_up.load(Ordering::Relaxed) {
            interrupts::enable_and_hlt();
            woke_up.store(false, Ordering::Relaxed);
        } else {
            interrupts::enable();
        }
    }
}
