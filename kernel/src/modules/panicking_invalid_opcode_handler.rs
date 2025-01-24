use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_invalid_opcode_handler(_stack_frame: InterruptStackFrame) {
    panic!("Invalid opcode!");
}
