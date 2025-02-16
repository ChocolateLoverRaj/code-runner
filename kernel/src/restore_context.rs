use core::arch::asm;

use crate::context::Context;

#[inline(always)]
/// # Safety
/// Sets the registers to whatever the context is and does `iretq`
pub unsafe fn restore_context(context: &Context) -> ! {
    unsafe {
        asm!("mov rsp, {};\
        pop rbp; pop rax; pop rbx; pop rcx; pop rdx; pop rsi; pop rdi; pop r8; pop r9;\
        pop r10; pop r11; pop r12; pop r13; pop r14; pop r15; iretq;",
        in(reg) context);
    }
    unreachable!()
}
