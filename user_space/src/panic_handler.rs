use core::{fmt::Write, panic::PanicInfo};

use crate::syscall::{syscall_exit, syscall_print};

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    // TODO: Nicer error display
    let mut message = heapless::String::<100>::new();
    message.write_fmt(format_args!("{}", panic_info)).unwrap();
    syscall_print(&message).unwrap();
    syscall_exit();
}
