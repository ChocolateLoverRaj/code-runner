use core::mem::MaybeUninit;

use alloc::{boxed::Box, vec::Vec};
use spinning_top::Spinlock;
use x86_64::{
    structures::paging::{PhysFrame, Size4KiB},
    VirtAddr,
};

use crate::iopb_size::IOPB_SIZE;

#[repr(C, align(16))]
pub struct StackChunk([u8; 16]);

#[derive(Debug)]
pub struct UserTaskData {
    pub cr3: PhysFrame<Size4KiB>,
    pub kernel_stack: Box<[MaybeUninit<StackChunk>]>,
    /// The IO Bitmap
    pub iobp: [u8; IOPB_SIZE],
}

/// Kernel tasks will be added later
#[derive(Debug)]
pub enum TaskType {
    User(UserTaskData),
}

#[derive(Debug, Clone, Copy)]
pub struct ReadyToStartState {
    pub instruction_pointer: VirtAddr,
    pub stack_pointer: VirtAddr,
}

#[derive(Debug)]
pub enum TaskState {
    ReadyToStart(ReadyToStartState),
    Running,
}

#[derive(Debug)]
pub struct Task {
    pub task_type: TaskType,
    pub state: TaskState,
}

/// Tasks are arranged from highest priority first to lowest priority
pub static TASKS: Spinlock<Vec<Task>> = Spinlock::new(Vec::new());
