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

    loop {
        let n = 0xCA11u64;
        let arg1 = 10u64;
        let arg2 = 20u64;
        let arg3 = 30u64;
        let arg4 = 40u64;

        let mut ret: u64;
        unsafe {
            asm!(
                "syscall",
                inlateout("rax") n as u64 => ret,
                in("rdi") arg1,
                in("rsi") arg2,
                in("rdx") arg3,
                in("r10") arg4,
                out("rcx") _, // rcx is used to store old rip
                out("r11") _, // r11 is used to store old rflags
                options(nostack, preserves_flags)
            );
        }
        let ret = ret;
        asm!("nop");
    }
}
