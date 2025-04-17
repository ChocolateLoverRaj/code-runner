use alloc::format;
use core::panic::PanicInfo;

use crate::syscall::{syscall_exit, syscall_print};

#[cfg(not(test))]
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    // TODO: Nicer error display
    syscall_print(&format!("{}", panic_info));
    syscall_exit();
}
