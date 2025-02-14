#![cfg_attr(not(test), no_std)]

pub mod mem;
pub mod syscall;
pub mod syscall_output;
pub mod syscall_pointer;
pub mod syscall_print;
pub mod syscall_slice;
pub mod syscall_start_recording_keyboard;
pub mod syscall_take_frame_buffer;
