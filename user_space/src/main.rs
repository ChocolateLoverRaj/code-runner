#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(int_roundings)]
#![feature(maybe_uninit_slice)]
extern crate alloc;

pub mod allocator;
pub mod async_keyboard;
// pub mod demo_maze_roller_game;
// pub mod draw_rust;
// pub mod embedded_graphics_frame_buffer;
pub mod execute_future;
pub mod executor_context;
pub mod panic_handler;
pub mod syscall;

use alloc::format;
use async_keyboard::AsyncKeyboard;
use execute_future::execute_future;
use executor_context::ExecutorContext;
use futures::StreamExt;
use pc_keyboard::{layouts::Us104Key, HandleControl, Keyboard, ScancodeSet1};
use syscall::{syscall_exit, syscall_print};

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    syscall_print("Hello from user space 🚀!");
    unsafe { allocator::init() };
    let executor_context = ExecutorContext::default();
    execute_future(
        async {
            let mut async_keyboard = AsyncKeyboard::init(&executor_context);
            let mut keyboard = Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore);
            while let Some(c) = async_keyboard.next().await {
                if let Some(key_event) = keyboard.add_byte(c).unwrap() {
                    if let Some(key) = keyboard.process_keyevent(key_event) {
                        syscall_print(&format!("Key pressed: {:?}", key));
                    }
                }
            }
        },
        &executor_context,
    );
    syscall_exit();
}
