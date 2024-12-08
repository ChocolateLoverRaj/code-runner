use core::cell::UnsafeCell;

use x86_64::{
    structures::idt::{self, DivergingHandlerFuncWithErrCode, InterruptStackFrame},
    VirtAddr,
};

use super::tss::TssBuilder;

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

pub fn get_double_fault_entry(tss: &mut TssBuilder) -> idt::Entry<DivergingHandlerFuncWithErrCode> {
    let mut entry = idt::Entry::<DivergingHandlerFuncWithErrCode>::missing();
    let entry_options = entry.set_handler_fn(double_fault_handler);
    let stack_index = tss
        .add_interrupt_stack_table_entry({
            const STACK_SIZE: usize = 4096 * 5;
            const STACK: UnsafeCell<[u8; STACK_SIZE]> = UnsafeCell::new([0; STACK_SIZE]);

            let stack_start = VirtAddr::from_ptr(STACK.get());
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        })
        .unwrap();
    unsafe {
        entry_options.set_stack_index(stack_index as u16);
    }
    entry
}
