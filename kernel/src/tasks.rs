use core::sync::atomic::AtomicUsize;

use alloc::vec::Vec;
use common::permissions::Permissions;
use spinning_top::Spinlock;
use util::init_later::{InitLater, TryInitError};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{PhysFrame, Size4KiB},
    VirtAddr,
};

use crate::{boxed_stack::BoxedStack, syscalls::raw_syscall_handler::PushedRegisters};

#[derive(Debug)]
pub struct UserTaskData {
    pub cr3: PhysFrame<Size4KiB>,
    pub kernel_stack: BoxedStack,
    pub permissions: Permissions<'static>,
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
pub struct SavedSyscallState {
    pub pushed_registers: PushedRegisters,
    pub stack_pointer: u64,
}

#[derive(Debug)]
pub enum TaskState {
    ReadyToStart(ReadyToStartState),
    Running,
    WaitingUntilEvent(SavedSyscallState),
}

#[derive(Debug)]
pub struct KeyboardEventListener {
    pub task_id: usize,
    pub pending_interrupt_received: bool,
}

#[derive(Debug)]
pub struct Task {
    pub task_type: TaskType,
    pub state: TaskState,
    pub id: usize,
}

#[derive(Debug)]
pub struct Tasks {
    pub kernel_cr3: PhysFrame<Size4KiB>,
    pub tasks: Vec<Task>,
    pub keyboard_listener: Option<KeyboardEventListener>,
}

impl Tasks {
    pub fn from_current_cr3() -> Self {
        Self {
            kernel_cr3: Cr3::read().0,
            tasks: Default::default(),
            keyboard_listener: None,
        }
    }
}

/// Tasks are arranged from highest priority first to lowest priority
pub static TASKS: InitLater<Spinlock<Tasks>> = InitLater::uninit();

pub static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(0);

pub fn get_running_task(tasks: &mut Vec<Task>) -> Option<&mut Task> {
    tasks.iter_mut().find(|task| match task.state {
        TaskState::Running => true,
        _ => false,
    })
}

pub fn try_init_tasks() -> Result<&'static Spinlock<Tasks>, TryInitError> {
    TASKS.try_init(Spinlock::new(Tasks::from_current_cr3()))
}

#[derive(Debug, Default)]
pub struct CpuTaskData {
    pub current_task: Option<usize>,
    pub stack_to_delete: Option<BoxedStack>,
}
