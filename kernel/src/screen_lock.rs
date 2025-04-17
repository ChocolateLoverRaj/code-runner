use spinning_top::Spinlock;
use x86_64::structures::paging::{Page, Size4KiB};

pub struct TaskUsingScreen {
    pub id: usize,
    pub mapped_start: Page<Size4KiB>,
}

pub enum WhoIsUsingScreen {
    KernelLogger,
    Task(TaskUsingScreen),
}

pub static SCREEN_LOCK: Spinlock<Option<WhoIsUsingScreen>> = Spinlock::new(None);
