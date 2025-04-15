use x86_64::structures::idt::InterruptStackFrame;

use super::handle_user_mode_fault::handle_user_mode_fault;

pub extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    handle_user_mode_fault(
        &stack_frame,
        format_args!("Segment not present. Error code: {}", error_code),
    );
    panic!("Segment not present! Error code: {:?}", error_code);
}
