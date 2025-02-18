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

use async_keyboard::AsyncKeyboard;
use common::syscall_start_recording_keyboard::FullQueueBehavior;
use demo_maze_roller_game::demo_maze_roller_game;
use embedded_graphics_frame_buffer::FrameBufferDisplay;
use execute_future::execute_future;
use futures::{stream, StreamExt};
use syscall::{syscall_exit, syscall_print, syscall_take_frame_buffer};

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    allocator::init();
    let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    syscall_print("Playing Maze Roller Game!").unwrap();
    execute_future(demo_maze_roller_game(
        &mut FrameBufferDisplay::new(&mut frame_buffer),
        AsyncKeyboard::<256>::new(FullQueueBehavior::DropNewest).flat_map(stream::iter),
    ));
    syscall_exit();
}
