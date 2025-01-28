use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: General Protection\n{:#?}\nError code: {:?}",
        stack_frame, error_code
    );
}
