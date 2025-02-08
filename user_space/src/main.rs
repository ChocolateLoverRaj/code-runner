#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::{arch::asm, fmt::Write, panic::PanicInfo};

use common::{Syscall, SyscallSlice};

fn syscall_internal(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
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

fn syscall(syscall: &Syscall) -> u64 {
    let [input0, input1, input2, input3, input4, input5, input6] =
        syscall.serialize_to_input().unwrap();
    syscall_internal(input0, input1, input2, input3, input4, input5, input6)
}

#[unsafe(no_mangle)]
extern "C" fn _start() {
    let string = "Hello from User Space (written in Rust ofc)!";
    let mut count = 0;
    loop {
        let mut message = heapless::String::<100>::new();
        message
            .write_fmt(format_args!("{}. Counter: {}", string, count))
            .unwrap();
        syscall(&Syscall::Print(SyscallSlice::from_slice(
            message.as_bytes(),
        )));
        count += 1;
    }
}

#[panic_handler]
fn panic(_panic_info: &PanicInfo) -> ! {
    unsafe {
        asm!("ud2");
        loop {}
    }
}
