use core::fmt::Arguments;

use x86_64::{registers::segmentation::GS, structures::idt::InterruptStackFrame, PrivilegeLevel};

use crate::terminate_current_task::terminate_current_task;

pub fn handle_user_mode_fault(stack_frame: &InterruptStackFrame, fault_description: Arguments) {
    // TODO: Have a better way of handling faults when they happen in user mode, not just for a GP fault
    if stack_frame.code_segment.rpl() == PrivilegeLevel::Ring3 {
        unsafe { GS::swap() };
        log::warn!(
            "User space program caused a {}. Stack frame: {:#?}. Terminating process",
            fault_description,
            stack_frame,
        );
        terminate_current_task()
    }
}
