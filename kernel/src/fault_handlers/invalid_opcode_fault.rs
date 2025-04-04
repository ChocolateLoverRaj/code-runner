use x86_64::structures::idt::InterruptStackFrame;

use crate::fault_handlers::handle_user_mode_fault::handle_user_mode_fault;

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    handle_user_mode_fault(&stack_frame, format_args!("Invalid Opcode Fault"));
    panic!("Invalid opcode! {:#?}", stack_frame);
}
