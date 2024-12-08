use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_local_apic_error_interrupt_handler(
    stack_frame: InterruptStackFrame,
) {
    panic!("EXCEPTION: LAPIC ERROR\n{:#?}", stack_frame);
}
