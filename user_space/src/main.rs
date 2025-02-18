#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(int_roundings)]
#![feature(maybe_uninit_slice)]
extern crate alloc;

pub mod allocator;
pub mod async_keyboard;
pub mod demo_maze_roller_game;
pub mod draw_rust;
pub mod embedded_graphics_frame_buffer;
pub mod execute_future;
pub mod panic_handler;
pub mod syscall;
pub mod test_disable_interrupts;

use alloc::format;
use async_keyboard::AsyncKeyboard;
use common::syscall_start_recording_keyboard::FullQueueBehavior;
use draw_rust::draw_rust;
use embedded_graphics_frame_buffer::FrameBufferDisplay;
use execute_future::execute_future;
use futures::StreamExt;
use syscall::{syscall_exit, syscall_print, syscall_take_frame_buffer};

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    // test_disable_interrupts::test_disable_interrupts();

    allocator::init();

    let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    draw_rust(&mut FrameBufferDisplay::new(&mut frame_buffer));

    execute_future(async {
        let k = AsyncKeyboard::<256>::new(FullQueueBehavior::DropNewest);
        k.for_each(move |scan_codes| async move {
            syscall_print(&format!("async scan codes: {:x?}", scan_codes)).unwrap();
        })
        .await;
    });
    syscall_exit();
}
