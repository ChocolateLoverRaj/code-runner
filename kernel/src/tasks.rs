use core::{mem::MaybeUninit, ops::Range, sync::atomic::AtomicUsize};

use alloc::{boxed::Box, vec::Vec};
use spinning_top::Spinlock;
use util::{
    continuous_bool_vec::ContinuousBoolVec,
    init_later::{InitLater, TryGetError, TryInitError},
};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::{
    cpu_local::CpuLocal,
    iopb_size::IOPB_SIZE,
    modules::syscall::{
        init_syscalls::{init_syscalls, InitializedSyscalls},
        syscall_handler_closure::set_syscall_handler_closure,
    },
    syscall_handler_closure::syscall_handler_closure,
};

#[derive(Debug)]
pub struct UserTaskData {
    pub cr3: PhysFrame<Size4KiB>,
    pub kernel_stack: Box<[MaybeUninit<StackChunk>]>,
    /// The IO Bitmap
    pub iopb: [u8; IOPB_SIZE],
    pub log_stream: Option<Range<PhysAddr>>,
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
    pub owned_phys_mem: ContinuousBoolVec<Vec<usize>>,
    pub id: usize,
}

#[derive(Debug)]
pub struct Tasks {
    pub kernel_cr3: PhysFrame<Size4KiB>,
    pub tasks: Vec<Task>,
}

impl Tasks {
    pub fn from_current_cr3() -> Self {
        Self {
            kernel_cr3: Cr3::read().0,
            tasks: Default::default(),
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
    pub initialized_syscalls: Option<InitializedSyscalls>,
    pub current_task: Option<usize>,
    pub stack_to_delete: Option<Box<[MaybeUninit<StackChunk>]>>,
}

static CPU_TASK_DATA: CpuLocal<Spinlock<CpuTaskData>> = CpuLocal::uninit();

pub fn try_init_cpu_task_data() -> Result<(), TryInitError> {
    CPU_TASK_DATA.try_init(Default::default)
}

pub fn try_get_cpu_task_data() -> Result<&'static Spinlock<CpuTaskData>, TryGetError> {
    CPU_TASK_DATA.try_get()
}

pub fn init_cpu_local_task_data() {
    try_get_cpu_task_data().unwrap().lock().initialized_syscalls = Some(init_syscalls(
        set_syscall_handler_closure(Box::new(syscall_handler_closure())),
    ));
}
