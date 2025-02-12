use x86_64::structures::{gdt::SegmentSelector, idt::InterruptStackFrame};

pub extern "x86-interrupt" fn panicking_general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let s = SegmentSelector(error_code.try_into().unwrap());
    panic!(
        "EXCEPTION: General Protection\n{:#?}\nError code: {:?}. Error code as segment selector: {:?}",
        stack_frame, error_code, s
    );
}
