use x86_64::{
    registers::segmentation::GS,
    structures::{gdt::SegmentSelector, idt::InterruptStackFrame},
    PrivilegeLevel,
};

pub extern "x86-interrupt" fn panicking_general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    // TODO: Have a better way of handling faults when they happen in user mode, not just for a GP fault
    if stack_frame.code_segment.rpl() == PrivilegeLevel::Ring3 {
        unsafe { GS::swap() };
    }
    let s = SegmentSelector(error_code.try_into().unwrap());
    panic!(
        "EXCEPTION: General Protection\n{:#?}\nError code: {:?}. Error code as segment selector: {:?}",
        stack_frame, error_code, s
    );
}
