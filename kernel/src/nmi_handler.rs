use x86_64::structures::idt::InterruptStackFrame;

use crate::hlt_loop::hlt_loop;

pub extern "x86-interrupt" fn nmi_handler(_interrupt_stack_frame: InterruptStackFrame) {
    // We cannot log here because the logger could be locked
    // Normally, interrupt are disabled when logging, so the logger is not locked during interrupt handlers
    // However, this is a non-maskable interrupt handler, which means that this handler can be called even while interrupts are disabled
    hlt_loop()
}
