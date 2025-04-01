use core::arch::asm;
use x86_64::{registers::rflags::RFlags, VirtAddr};

use super::init_syscalls::InitializedSyscalls;

#[derive(Debug, Clone, Copy)]
pub struct EnterUserModeInput {
    pub initialized_syscalls: InitializedSyscalls,
    pub code: VirtAddr,
    pub stack_end: VirtAddr,
    pub rflags: RFlags,
}

/// Does `swapgs` and then enters user mode using the `sysret` instruction
///
/// # Safety
/// Jumps to an unchecked address with an unchecked stack.
/// You should handle any exceptions that happen in Ring3 and not crash the kernel because of exception in Ring3.
pub unsafe fn enter_user_mode(
    EnterUserModeInput {
        initialized_syscalls: _,
        code,
        stack_end,
        rflags,
    }: EnterUserModeInput,
) -> ! {
    // Based on https://wiki.osdev.org/Getting_to_Ring_3#sysret_method
    // 0x0002 should always be set
    // https://en.wikipedia.org/wiki/FLAGS_register
    // "Reserved, always 1 in EFLAGS"
    let rip = code.as_u64();
    let rsp = stack_end.as_u64();
    let rflags = rflags.bits();
    unsafe {
        asm!("\
            swapgs
            mov rsp, {}
            sysretq",
            in(reg) rsp,
            in("rcx") rip,
            in("r11") rflags
        );
    }
    unreachable!()
}
