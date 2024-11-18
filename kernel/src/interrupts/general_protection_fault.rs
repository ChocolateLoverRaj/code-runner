use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
    panic!("EXCEPTION: General Protection\n{:#?}", stack_frame);
}
