#![feature(trivial_bounds)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod mem;
pub mod permissions;
pub mod ram_disk;
pub mod syscall;
pub mod syscall_output;
pub mod syscall_pointer;
pub mod syscall_print;
pub mod syscall_slice;
pub mod syscall_start_recording_keyboard;
pub mod syscall_take_frame_buffer;
pub mod syscall_uuids;
