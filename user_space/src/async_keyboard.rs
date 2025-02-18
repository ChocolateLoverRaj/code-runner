use core::{mem::MaybeUninit, task::Poll};

use common::syscall_start_recording_keyboard::{
    FullQueueBehavior, SyscallStartRecordingKeyboardInput,
};
use futures::{task::AtomicWaker, Stream};

use crate::syscall::{
    syscall_done_with_interrupt_handler, syscall_poll_keyboard,
    syscall_set_keyboard_interrupt_handler, syscall_start_recording_keyboard,
};

static WAKER: AtomicWaker = AtomicWaker::new();

pub struct AsyncKeyboard<const T: usize> {}

impl<const N: usize> AsyncKeyboard<N> {
    const QUEUE_SIZE: usize = N;
    pub fn new(full_queue_behavior: FullQueueBehavior) -> Self {
        syscall_start_recording_keyboard(SyscallStartRecordingKeyboardInput {
            queue_size: Self::QUEUE_SIZE as u64,
            behavior_on_full_queue: full_queue_behavior,
        });
        syscall_set_keyboard_interrupt_handler(Some(keyboard_interrupt_handler));
        Self {}
    }
}

impl<const N: usize> Drop for AsyncKeyboard<N> {
    fn drop(&mut self) {
        syscall_set_keyboard_interrupt_handler(None);
        todo!("Tell kernel to stop recording keyboard");
    }
}

impl<const N: usize> Stream for AsyncKeyboard<N> {
    type Item = heapless::Vec<u8, N>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        WAKER.register(cx.waker());
        let mut buffer = [MaybeUninit::uninit(); N];
        let scan_codes = syscall_poll_keyboard(&mut buffer);
        if !scan_codes.is_empty() {
            Poll::Ready(Some(heapless::Vec::from_slice(scan_codes).unwrap()))
        } else {
            Poll::Pending
        }
    }
}

unsafe extern "sysv64" fn keyboard_interrupt_handler() -> ! {
    // We cannot allocate during the interrupt handler because that would cause a lock forever.
    // Same reason why we can't allocate in kernel interrupt handlers
    // for _ in 0..1_000_000 {}
    // syscall_print("Keyboard interrupt handler").unwrap();
    WAKER.wake();
    syscall_done_with_interrupt_handler();
}
