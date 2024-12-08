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

pub extern "x86-interrupt" fn panicking_page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    let page_fault_error = PageFaultError {
        accessed_address: Cr2::read(),
        error_code,
        stack_frame,
    };

    panic!("Page fault: {page_fault_error:#?}");
}
