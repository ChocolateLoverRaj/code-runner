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
use syscall::{syscall_exists, syscall_exit, syscall_print, syscall_test};
use uuid::Uuid;

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
    syscall_exit();

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
