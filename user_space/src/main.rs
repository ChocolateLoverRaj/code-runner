#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod syscall;

use core::{fmt::Write, panic::PanicInfo};

use syscall::{syscall_print, syscall_take_frame_buffer};

#[unsafe(no_mangle)]
extern "C" fn _start() {
    let mut count: u64 = 0;
    let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    loop {
        frame_buffer
            .buffer_mut()
            .fill((count % u8::MAX as u64) as u8);
        let mut message = heapless::String::<100>::new();
        message
            .write_fmt(format_args!("Hello from user space. Counter: {}", count))
            .unwrap();
        syscall_print(&message).unwrap();
        count += 1;
    }
}

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    // TODO: Nicer error display
    let mut message = heapless::String::<100>::new();
    message.write_fmt(format_args!("{}", panic_info)).unwrap();
    syscall_print(&message).unwrap();
    // TODO: Exit process
    loop {}
}
