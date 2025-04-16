use core::fmt::Debug;

use x86_64::{
    addr::VirtAddrNotValid,
    registers::control::{Cr2, Cr3, Cr3Flags},
    structures::{
        idt::{InterruptStackFrame, PageFaultErrorCode},
        paging::PhysFrame,
    },
    VirtAddr,
};

use super::handle_user_mode_fault::handle_user_mode_fault;

#[derive(Debug)]
#[allow(unused)]
struct PageFaultError<'a> {
    accessed_address: Result<VirtAddr, VirtAddrNotValid>,
    error_code: PageFaultErrorCode,
    stack_frame: &'a InterruptStackFrame,
    cr3: (PhysFrame, Cr3Flags),
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let page_fault_error = PageFaultError {
        accessed_address: Cr2::read(),
        error_code,
        stack_frame: &stack_frame,
        cr3: Cr3::read(),
    };
    handle_user_mode_fault(&stack_frame, format_args!("{:#?}", page_fault_error));
    panic!("Page fault: {page_fault_error:#?}");
}
