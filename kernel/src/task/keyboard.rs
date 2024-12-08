// use core::{
//     pin::Pin,
//     task::{Context, Poll},
// };

// use conquer_once::spin::OnceCell;
// use crossbeam_queue::ArrayQueue;
// use futures_util::{task::AtomicWaker, Stream};

// use crate::keyboard_interrupt_mutex::{KeyboardInterruptLock, KEYBOARD_INTERRUPT_MUTEX};

// static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
// static WAKER: AtomicWaker = AtomicWaker::new();

// /// Called by the keyboard interrupt handler
// ///
// /// Must not block or allocate.
// pub(crate) fn add_scancode(scancode: u8) {
//     match SCANCODE_QUEUE.try_get() {
//         Ok(queue) => match queue.push(scancode) {
//             Ok(()) => {
//                 WAKER.wake();
//             }
//             Err(e) => {
//                 log::warn!("WARNING: scancode queue full; dropping keyboard input: {e:?}");
//             }
//         },
//         Err(e) => {
//             log::warn!("WARNING: Could not add scancode: {e:?}");
//         }
//     }
// }

// pub struct ScancodeStream {
//     _keyboard_interrupt_lock: KeyboardInterruptLock,
// }

// impl ScancodeStream {
//     pub fn new() -> Option<Self> {
//         KEYBOARD_INTERRUPT_MUTEX
//             .try_lock()
//             .map(|mut keyboard_interrupt_lock| {
//                 match SCANCODE_QUEUE.get() {
//                     Some(scancode_queue) => loop {
//                         let popped_value = scancode_queue.pop();
//                         if popped_value.is_none() {
//                             break;
//                         }
//                     },
//                     None => SCANCODE_QUEUE.init_once(|| ArrayQueue::new(100)),
//                 };
//                 keyboard_interrupt_lock.enable();
//                 ScancodeStream {
//                     _keyboard_interrupt_lock: keyboard_interrupt_lock,
//                 }
//             })
//     }
// }

// impl Stream for ScancodeStream {
//     type Item = u8;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
//         let queue = SCANCODE_QUEUE
//             .try_get()
//             .expect("scancode queue not initialized");

//         // fast path
//         if let Some(scancode) = queue.pop() {
//             return Poll::Ready(Some(scancode));
//         }

//         WAKER.register(&cx.waker());
//         match queue.pop() {
//             Some(scancode) => {
//                 WAKER.take();
//                 Poll::Ready(Some(scancode))
//             }
//             None => Poll::Pending,
//         }
//     }
// }
