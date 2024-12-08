use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_spurious_interrupt_handler(
    stack_frame: InterruptStackFrame,
) {
    panic!("EXCEPTION: SPURIOUS INTERRUPT\n{:#?}", stack_frame);
}
