use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}
