use core::mem::MaybeUninit;

use x86_64::{
    structures::{
        gdt::SegmentSelector,
        idt::{self, DivergingHandlerFuncWithErrCode},
    },
    VirtAddr,
};

use crate::tasks::StackChunk;

use super::tss::TssBuilder;

pub fn get_double_fault_entry<const N: usize>(
    tss: &mut TssBuilder<N>,
    handler: DivergingHandlerFuncWithErrCode,
    cs: SegmentSelector,
    stack: &'static mut [MaybeUninit<StackChunk>],
) -> idt::Entry<DivergingHandlerFuncWithErrCode> {
    idt::Entry::from_handler_fn(handler, {
        let mut options = idt::EntryOptions::present_with_cs(cs);
        let stack_index = tss
            .add_interrupt_stack_table_entry(VirtAddr::from_ptr(stack.as_ptr_range().end))
            .unwrap();
        unsafe { options.set_stack_index(stack_index) };
        options
    })
}
