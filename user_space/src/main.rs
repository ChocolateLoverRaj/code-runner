#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(int_roundings)]
extern crate alloc;

pub mod allocator;
pub mod demo_maze_roller_game;
pub mod draw_rust;
pub mod embedded_graphics_frame_buffer;
pub mod execute_future;
pub mod panic_handler;
pub mod syscall;

use alloc::format;
use common::syscall_start_recording_keyboard::{
    FullQueueBehavior, SyscallStartRecordingKeyboardInput,
};
use draw_rust::draw_rust;
use embedded_graphics_frame_buffer::FrameBufferDisplay;
use syscall::{
    syscall_block_until_event, syscall_done_with_interrupt_handler, syscall_poll_keyboard,
    syscall_print, syscall_set_keyboard_interrupt_handler, syscall_start_recording_keyboard,
    syscall_take_frame_buffer,
};

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    allocator::init();

    // let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    // draw_rust(&mut FrameBufferDisplay::new(&mut frame_buffer));
    syscall_start_recording_keyboard(SyscallStartRecordingKeyboardInput {
        queue_size: 256,
        behavior_on_full_queue: FullQueueBehavior::DropNewest,
    });
    syscall_set_keyboard_interrupt_handler(Some(keyboard_interrupt_handler));
    let mut buffer = [Default::default(); 256];
    loop {
        syscall_print("Blocking until event").unwrap();
        syscall_block_until_event();
        syscall_print("Done blocking until event").unwrap();
        let scan_codes = syscall_poll_keyboard(&mut buffer);
        if !scan_codes.is_empty() {
            syscall_print(&format!("Got scan codes: {:x?}", scan_codes)).unwrap();
        }
    }
}

static mut C: u64 = 0;
unsafe extern "sysv64" fn keyboard_interrupt_handler() -> ! {
    syscall_print(&format!("Got keyboard interrupt; {:?}", unsafe { C })).unwrap();
    unsafe { C += 1 };
    syscall_done_with_interrupt_handler();
}
