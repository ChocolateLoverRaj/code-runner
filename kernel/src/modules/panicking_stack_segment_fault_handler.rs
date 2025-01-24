use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_stack_segment_fault_handler(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("Stack segment faul! Error code: {:?}", error_code);
}
