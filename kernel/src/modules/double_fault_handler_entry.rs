use x86_64::{
    structures::idt::{self, DivergingHandlerFuncWithErrCode},
    VirtAddr,
};

use super::tss::TssBuilder;

pub fn get_double_fault_entry(
    tss: &mut TssBuilder,
    handler: DivergingHandlerFuncWithErrCode,
) -> idt::Entry<DivergingHandlerFuncWithErrCode> {
    let mut entry = idt::Entry::missing();
    let entry_options = entry.set_handler_fn(handler);
    let stack_index = tss
        .add_interrupt_stack_table_entry({
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe {
                #[allow(static_mut_refs)]
                STACK.as_mut_ptr()
            });
            stack_start + STACK_SIZE as u64
        })
        .unwrap();
    unsafe {
        entry_options.set_stack_index(stack_index as u16);
    }
    entry
}
