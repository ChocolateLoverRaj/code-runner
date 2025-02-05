#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::{arch::asm, panic::PanicInfo};

pub fn sycall(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64, arg6: u64) -> u64 {
    let return_value: u64;
    unsafe {
        asm!("\
            mov rdi, {0}
            mov rsi, {1}
            mov rdx, {2}
            mov r10, {3}
            mov r8,  {4}
            mov r9,  {5}
            mov rax, {6}
            syscall
            ",
            in(reg) arg0,
            in(reg) arg1,
            in(reg) arg2,
            in(reg) arg3,
            in(reg) arg4,
            in(reg) arg5,
            in(reg) arg6,
            lateout("rax") return_value
        );
    }
    return_value
}

#[unsafe(no_mangle)]
extern "C" fn _start() {
    let mut c = 0;
    loop {
        let _a = sycall(0x10, 0x20, 0x30, 0x40, 0x50, 0x60, c);
        c += 1;
    }
}

#[panic_handler]
fn panic(_panic_info: &PanicInfo) -> ! {
    unsafe {
        asm!("ud2");
        loop {}
    }
}
