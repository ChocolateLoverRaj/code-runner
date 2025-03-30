use x86_64::structures::idt::InterruptStackFrame;

use crate::cpu_local_data::get_local;

pub extern "x86-interrupt" fn logging_breakpoint_handler(stack_frame: InterruptStackFrame) {
    let local_data = unsafe { get_local().get().read() };
    log::info!(
        "EXCEPTION: BREAKPOINT\n{:#?} {:#?}",
        stack_frame,
        local_data
    );
}
