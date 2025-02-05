#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo};

#[unsafe(no_mangle)]
extern "C" fn _start() {
    unsafe {
        asm!(
            "\
        mov rbx, 0
        2:
        push 0x595ca11b // keep the syscall number in the stack
        mov rbp, 0x100 // distinct values for each register
        mov rax, 0x101
        mov rcx, 0x103
        mov rdx, 0x104
        mov rdi, 0x106
        mov r8, 0x107
        mov r9, 0x108
        mov r10, 0x109
        mov r11, 0x110
        mov r12, 0x111
        mov r13, 0x112
        mov r14, 0x113
        mov r15, 0x114
        xor rax, rax
        3:
        inc rax
        cmp rax, 0x4000000
        jnz 3b // loop for some milliseconds
        pop rax // pop syscall number from the stack
        inc rbx // increase loop counter
        mov rdi, rsp // first syscall arg is rsp
        mov rsi, rbx // second syscall arg is the loop counter
        syscall // perform the syscall!
        jmp 2b // do it all over
    "
        )
    }
}

#[panic_handler]
fn panic(_panic_info: &PanicInfo) -> ! {
    unsafe {
        asm!("ud2");
        loop {}
    }
}
