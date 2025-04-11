use core::mem::MaybeUninit;

use alloc::boxed::Box;
use x86_64::VirtAddr;

#[repr(C, align(16))]
pub struct StackChunk([u8; 16]);

pub type BoxedStack = Box<[MaybeUninit<StackChunk>]>;

pub trait BoxedStackExt {
    fn new_uninit_stack(stack_size_bytes: usize) -> Self;
    fn starting_stack_pointer(&self) -> VirtAddr;
}

impl BoxedStackExt for BoxedStack {
    fn new_uninit_stack(stack_size_bytes: usize) -> Self {
        Box::new_uninit_slice(stack_size_bytes / 16)
    }

    fn starting_stack_pointer(&self) -> VirtAddr {
        VirtAddr::from_ptr(self.as_ptr_range().end)
    }
}
