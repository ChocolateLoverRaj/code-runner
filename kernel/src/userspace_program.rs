use core::arch::asm;

pub unsafe extern "C" fn userspace_program() {
    // asm!(
    //     "2:
    //     mov rax, 0xCA11
    //     mov rdi, 10
    //     mov rsi, 20
    //     mov rdx, 30
    //     mov r10, 40
    //     syscall
    //     jmp 2b",
    //     options(nostack, preserves_flags)
    // );

    // loop {
    //     let n = 0xCA11u64;
    //     let arg1 = 10u64;
    //     let arg2 = 20u64;
    //     let arg3 = 30u64;
    //     let arg4 = 40u64;

    //     let mut ret: u64;
    //     unsafe {
    //         asm!(
    //             "syscall",
    //             inlateout("rax") n as u64 => ret,
    //             in("rdi") arg1,
    //             in("rsi") arg2,
    //             in("rdx") arg3,
    //             in("r10") arg4,
    //             out("rcx") _, // rcx is used to store old rip
    //             out("r11") _, // r11 is used to store old rflags
    //             options(nostack, preserves_flags)
    //         );
    //     }
    //     let ret = ret;
    //     asm!("nop");
    // }

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
