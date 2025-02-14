#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod draw_rust;
pub mod embedded_graphics_frame_buffer;
pub mod panic_handler;
pub mod syscall;

use draw_rust::draw_rust;
use syscall::{syscall_exit, syscall_take_frame_buffer};

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    draw_rust(&mut frame_buffer);
    syscall_exit()
}
