use core::arch::asm;
use x86_64::{registers::rflags::RFlags, VirtAddr};

/// # Safety
/// Jumps to an unchecked address with an unchecked stack.
/// You should handle any exceptions that happen in Ring3 and not crash the kernel because of exception in Ring3.
pub unsafe fn enter_user_mode(code: VirtAddr, stack_end: VirtAddr) {
    // Based on https://wiki.osdev.org/Getting_to_Ring_3#sysret_method
    // 0x0002 should always be set
    // https://en.wikipedia.org/wiki/FLAGS_register
    // "Reserved, always 1 in EFLAGS"
    let eflags = RFlags::INTERRUPT_FLAG.bits() | 0x0002;
    let rip = code.as_u64();
    let rsp = stack_end.as_u64();
    unsafe {
        asm!("\
            mov rsp, {}
            sysretq",
            in(reg) rsp,
            in("rcx") rip,
            in("r11") eflags
        );
    }
}
