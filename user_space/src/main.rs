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

use core::fmt::Write;

use async_keyboard::AsyncKeyboard;
use common::{syscall_start_recording_keyboard::FullQueueBehavior, syscall_uuids::SYSCALL_EXIT};
use demo_maze_roller_game::demo_maze_roller_game;
use embedded_graphics_frame_buffer::FrameBufferDisplay;
use execute_future::execute_future;
use futures::{stream, StreamExt};
use syscall::{
    syscall_enable_hpet, syscall_exists, syscall_exit, syscall_get_hpet_main_counter_period,
    syscall_hpet_read_main_counter_value, syscall_internal, syscall_print,
    syscall_take_frame_buffer, syscall_uuid,
};
use uart_16550::uart_16550::Uart16550;
use uuid::Uuid;
use x86_64::instructions::port::Port;

/// Blocks until the given amount of femtoseconds have passed
pub fn spin_fs(duration_fs: u128) {
    // Doesn't hurt to enable if it's already enabled
    syscall_enable_hpet();
    let period_fs = syscall_get_hpet_main_counter_period();
    let counter_before = syscall_hpet_read_main_counter_value();
    loop {
        let counter_now = syscall_hpet_read_main_counter_value();
        let elapsed_fs = (counter_now - counter_before) as u128 * period_fs as u128;
        if elapsed_fs >= duration_fs {
            break;
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    let can_exit = syscall_exists(SYSCALL_EXIT);
    let should_be_false = syscall_exists(Uuid::default());
    let mut count = 0;

    // let v = unsafe { Port::<u8>::new(0x3F8).read() };

    let mut port = unsafe { uart_16550::port::new(0x3F8) };
    port.init();
    port.write_str("Hello from User space! User space has taken over the UART!\n");
    loop {
        let n = port.receive();
        write!(port, "Received: {:?}\n", core::str::from_utf8(&[n]));
    }

    loop {
        let return_value = unsafe { syscall_uuid(SYSCALL_EXIT, [100, 101, 102, 103, 104]) };
        count += 1;
    }
    // allocator::init();

    // // let duration = 3 * 10_u128.pow(15);
    // // syscall_print(&format!("Spinning for {} fs", duration)).unwrap();
    // // spin_fs(duration);
    // // syscall_print("Done spinning").unwrap();

    // let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    // syscall_print("Playing Maze Roller Game!").unwrap();
    // execute_future(demo_maze_roller_game(
    //     &mut FrameBufferDisplay::new(&mut frame_buffer),
    //     AsyncKeyboard::<256>::new(FullQueueBehavior::DropNewest).flat_map(stream::iter),
    // ));
    // syscall_exit();
}
