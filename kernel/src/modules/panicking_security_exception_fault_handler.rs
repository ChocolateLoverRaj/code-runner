use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_invalid_tss_fault_handler(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("Security exception. Error code: {:?}", error_code);
}
