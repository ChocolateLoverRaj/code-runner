use x86_64::structures::idt::InterruptStackFrame;

pub extern "x86-interrupt" fn panicking_invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("Invalid opcode! {:#?}", stack_frame);
}
