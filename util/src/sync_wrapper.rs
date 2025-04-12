use core::cell::UnsafeCell;

pub struct SyncWrapper<T> {
    data: UnsafeCell<T>,
}

unsafe impl<T> Sync for SyncWrapper<T> {}

impl<T> SyncWrapper<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }

    /// # Safety
    /// You must ensure that the data inside doesn't get accessed by more than one thread
    pub unsafe fn get(&self) -> &T {
        unsafe { self.data.get().as_ref() }.unwrap()
    }
}
