use core::{sync::atomic::Ordering, task::Poll};

use futures::Stream;
use x86_64::instructions::port::Port;

use crate::{
    executor_context::ExecutorContext,
    syscall::{syscall_listen_for_keyboard_interrupts, syscall_take_io_port},
};

pub struct AsyncKeyboard<'a> {
    executor_context: &'a ExecutorContext,
}

impl<'a> AsyncKeyboard<'a> {
    pub fn init(executor_context: &'a ExecutorContext) -> Self {
        syscall_take_io_port(0x60).unwrap();
        syscall_listen_for_keyboard_interrupts();
        Self { executor_context }
    }
}

impl Drop for AsyncKeyboard<'_> {
    fn drop(&mut self) {
        todo!("Tell kernel to stop recording keyboard");
    }
}

impl Stream for AsyncKeyboard<'_> {
    type Item = u8;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        self.executor_context.keyboard_waker.register(cx.waker());
        match self
            .executor_context
            .keyboard_event_received
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Poll::Ready(Some(unsafe { Port::<u8>::new(0x60).read() })),
            Err(_) => Poll::Pending,
        }
    }
}
