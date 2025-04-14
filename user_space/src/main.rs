#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(int_roundings)]
#![feature(maybe_uninit_slice)]
// extern crate alloc;

pub mod allocator;
// pub mod async_keyboard;
// pub mod demo_maze_roller_game;
// pub mod draw_rust;
// pub mod embedded_graphics_frame_buffer;
// pub mod execute_future;
pub mod panic_handler;
pub mod syscall;
// pub mod test_disable_interrupts;

use common::syscall_uuids::{Syscall, SyscallExit};
use heapless::String;
use syscall::{
    syscall_exists, syscall_listen_for_keyboard_interrupts, syscall_print, syscall_take_io_port,
    syscall_test, syscall_wait_until_event,
};
use uuid::Uuid;
use x86_64::instructions::port::Port;

use core::fmt::Write;
// /// Blocks until the given amount of femtoseconds have passed
// pub fn spin_fs(duration_fs: u128) {
//     // Doesn't hurt to enable if it's already enabled
//     syscall_enable_hpet();
//     let period_fs = syscall_get_hpet_main_counter_period();
//     let counter_before = syscall_hpet_read_main_counter_value();
//     loop {
//         let counter_now = syscall_hpet_read_main_counter_value();
//         let elapsed_fs = (counter_now - counter_before) as u128 * period_fs as u128;
//         if elapsed_fs >= duration_fs {
//             break;
//         }
//     }
// }

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    syscall_test();
    // unsafe {
    //     asm!("ud2");
    // }
    // unsafe { (1 as *const u8).read_volatile() };
    syscall_print("Hello from user space 🚀!");
    let can_exit = syscall_exists(&SyscallExit::UUID);
    assert_eq!(can_exit, true);
    let should_be_false = syscall_exists(&Uuid::default());
    assert_eq!(should_be_false, false);

    syscall_take_io_port(0x60).unwrap();
    syscall_listen_for_keyboard_interrupts();
    let mut port = Port::<u8>::new(0x60);
    loop {
        syscall_wait_until_event();
        let data = unsafe { port.read() };
        if data != 250 {
            let mut message = String::<128>::new();
            write!(message, "Received data: {}", data);
            syscall_print(&message);
        }
    }
    // syscall_exit();

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
