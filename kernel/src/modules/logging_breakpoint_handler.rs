use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn logging_breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::info!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
