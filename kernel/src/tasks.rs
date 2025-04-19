use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::permissions::Permissions;
use spinning_top::Spinlock;
use util::init_later::InitLater;
use x86_64::{
    structures::paging::{PhysFrame, Size4KiB},
    VirtAddr,
};

use crate::{
    boxed_stack::BoxedStack, context::FullContext, syscalls::raw_syscall_handler::PushedRegisters,
};

#[derive(Debug, Clone, Copy)]
pub struct ReadyToStartState {
    pub instruction_pointer: VirtAddr,
    pub stack_pointer: VirtAddr,
}

#[derive(Debug)]
pub struct SavedSyscallState {
    pub pushed_registers: PushedRegisters,
    pub stack_pointer: u64,
}

#[derive(Debug)]
pub enum TaskState {
    ReadyToStart(ReadyToStartState),
    Running,
    WaitingUntilEvent(SavedSyscallState),
    /// A task's state can be interrupted when the kernel receives an interrupt and switches to a higher priority task.
    Interrupted(FullContext),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(usize);
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(0);
impl TaskId {
    pub fn new_unique() -> Self {
        Self(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct KeyboardEventListener {
    pub task_id: TaskId,
    pub pending_interrupt_received: bool,
}

#[derive(Debug)]
pub struct Task {
    pub state: TaskState,
    pub id: TaskId,
    pub cr3: PhysFrame<Size4KiB>,
    pub kernel_stack: BoxedStack,
    pub permissions: Permissions<'static>,
}

/// Tasks are arranged from highest priority first to lowest priority
pub static TASKS: Spinlock<BTreeMap<TaskId, Task>> = Spinlock::new(BTreeMap::new());
pub static TASK_QUEUE: Spinlock<Vec<TaskId>> = Spinlock::new(Vec::new());
pub static KERNEL_CR3: InitLater<PhysFrame<Size4KiB>> = InitLater::uninit();
pub static KEYBOARD_LISTENER: Spinlock<Option<KeyboardEventListener>> = Spinlock::new(None);

#[derive(Debug, Default)]
pub struct CpuTaskData {
    pub current_task: Option<TaskId>,
    pub stack_to_delete: Option<BoxedStack>,
}
