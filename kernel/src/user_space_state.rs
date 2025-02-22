use alloc::vec::Vec;
// use x86_64::VirtAddr;

use crate::context::AnyContext;

// For now since we don't have any kernel tasks and only have 1 user space task this can just be an `Option` instead of a list of tasks
#[derive(Debug)]
pub struct UserSpaceState {
    // TODO: Don't use a fixed size vec
    pub stack_of_saved_contexts: Vec<AnyContext>,
    // /// During a syscall, this is set to the stack pointer of the user space stack so that user space interrupt handlers can be called on their own stack instead of the kernel's sycall handler stack
    // pub stack_pointer: Option<VirtAddr>,
    pub keyboard_interrupt_queued: bool,
    pub in_keyboard_interrupt_handler: bool,
    pub interrupts_enabled: bool,
}

pub type State = Option<UserSpaceState>;
