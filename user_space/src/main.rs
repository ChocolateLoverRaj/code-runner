#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod draw_rust;
pub mod embedded_graphics_frame_buffer;
pub mod panic_handler;
pub mod syscall;

use core::fmt::Write;

use common::syscall_start_recording_keyboard::{
    FullQueueBehavior, SyscallStartRecordingKeyboardInput,
};
use draw_rust::draw_rust;
use syscall::{
    syscall_poll_keyboard, syscall_print, syscall_start_recording_keyboard,
    syscall_take_frame_buffer,
};

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    draw_rust(&mut frame_buffer);
    syscall_start_recording_keyboard(SyscallStartRecordingKeyboardInput {
        queue_size: 256,
        behavior_on_full_queue: FullQueueBehavior::DropNewest,
    });
    let mut buffer = [Default::default(); 256];
    loop {
        // To test getting multiple scan codes at once
        spin(10_000_000);
        let scan_codes = syscall_poll_keyboard(&mut buffer);
        if !scan_codes.is_empty() {
            syscall_print(&{
                let mut message = heapless::String::<1000>::new();
                message
                    .write_fmt(format_args!("Got scan codes: {:x?}", scan_codes))
                    .unwrap();
                message
            })
            .unwrap();
        }
    }
}

fn spin(count: usize) {
    for _ in 0..count {
        x86_64::instructions::nop();
    }
}
