use spinning_top::Spinlock;

pub enum WhoIsUsingScreen {
    KernelLogger,
    Task(usize),
}

pub static SCREEN_LOCK: Spinlock<Option<WhoIsUsingScreen>> = Spinlock::new(None);
