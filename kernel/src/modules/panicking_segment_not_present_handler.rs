use core::fmt::Debug;

use x86_64::{
    addr::VirtAddrNotValid,
    structures::idt::{InterruptStackFrame, PageFaultErrorCode},
    VirtAddr,
};

#[derive(Debug)]
#[allow(unused)]
struct PageFaultError {
    accessed_address: Result<VirtAddr, VirtAddrNotValid>,
    error_code: PageFaultErrorCode,
    stack_frame: InterruptStackFrame,
}

pub extern "x86-interrupt" fn panicking_segment_not_present_handler(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("Segment not present! Error code: {:?}", error_code);
}
