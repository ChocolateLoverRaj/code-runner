use core::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use alloc::{sync::Arc, task::Wake};
use futures::pin_mut;

use crate::syscall::{
    syscall_disable_and_defer_my_interrupts, syscall_enable_and_catch_up_on_my_interrupts,
    syscall_enable_my_interrupts_and_wait_until_one_happens,
};

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
/// Very similar to how the kernel does it
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
        // Disable interrupts here so that an interrupt doesn't happen in between checking if we woke up and getting woken up
        syscall_disable_and_defer_my_interrupts();
        if !woke_up.load(Ordering::Relaxed) {
            // Wait for an interrupt to happen
            syscall_enable_my_interrupts_and_wait_until_one_happens();
            woke_up.store(false, Ordering::Relaxed);
        } else {
            // We got woken up. Don't forget to enable interrupts again.
            syscall_enable_and_catch_up_on_my_interrupts();
        }
    }
}
