use spinning_top::Spinlock;
use x86_64::instructions::interrupts::without_interrupts;

pub struct LockWithoutInterrupts<T> {
    lock: Spinlock<T>,
}

impl<T> LockWithoutInterrupts<T> {
    pub const fn new(value: T) -> Self {
        Self {
            lock: Spinlock::new(value),
        }
    }
}

impl<T> LockWithoutInterrupts<T> {
    pub fn lock_without_interrupts<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        without_interrupts(|| {
            let mut guard = self.lock.lock();
            f(&mut guard)
        })
    }

    /// # Safety
    ///
    /// This method must only be called if the current thread logically owns a
    /// `MutexGuard` but that guard has been discarded using `mem::forget`.
    /// Behavior is undefined if a mutex is unlocked when not locked.
    pub unsafe fn force_unlock(&self) {
        unsafe { self.lock.force_unlock() };
    }
}
