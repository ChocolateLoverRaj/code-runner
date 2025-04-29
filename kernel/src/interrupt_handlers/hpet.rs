use core::arch::naked_asm;

use x86_64::{
    registers::segmentation::GS,
    structures::{gdt::SegmentSelector, idt::InterruptStackFrame},
    PrivilegeLevel,
};

use crate::context::FullContext;

/// # Safety
/// This function should only be called as a HPET interrupt handler, and not manually.
#[naked]
pub unsafe extern "sysv64" fn hpet_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        naked_asm!("\
            push r15
            push r14
            push r13
            push r12
            push r11
            push r10
            push r9
            push r8
            push rdi
            push rsi
            push rdx
            push rcx
            push rbx
            push rax
            push rbp

            mov rdi, rsp   // first arg of context switch is the context which is all the registers saved above

            // The function should never return
            call {context_switch}
            // asm! version of unreachable!()
            ud2
            ",
            context_switch = sym hpet_interrupt_handler_rust
        );
    };
}

unsafe extern "sysv64" fn hpet_interrupt_handler_rust(context: &FullContext) -> ! {
    // This is to make sure that GS.Base is set to the kernel's gs base
    let _swapped_gs =
        if SegmentSelector(context.cs.try_into().unwrap()).rpl() == PrivilegeLevel::Ring3 {
            unsafe { GS::swap() };
            log::trace!("Swapped GS");
            true
        } else {
            log::trace!("Didn't swap GS");
            false
        };

    todo!("HPET interrupt handler")
}
