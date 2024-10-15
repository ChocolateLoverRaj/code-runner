#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(code_runner::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use code_runner::{println, test_panic_handler};

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

#[test_case]
fn test_println() {
    println!("test_println output");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
