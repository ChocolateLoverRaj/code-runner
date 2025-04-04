use x86_64::structures::idt::InterruptStackFrame;

use crate::fault_handlers::handle_user_mode_fault::handle_user_mode_fault;

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    // Double fault handlers do not contain error codes
    _error_code: u64,
) -> ! {
    handle_user_mode_fault(&stack_frame, format_args!("Double Fault"));
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}
