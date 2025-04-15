use x86_64::structures::idt::InterruptStackFrame;

use super::handle_user_mode_fault::handle_user_mode_fault;

/// The General Protection Fault handler
pub extern "x86-interrupt" fn gp_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    handle_user_mode_fault(
        &stack_frame,
        format_args!("GP fault with error code: {}", error_code),
    );
    panic!(
        "EXCEPTION: General Protection\n{:#?}\nError code: {:?}",
        stack_frame, error_code
    );
}
