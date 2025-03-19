use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

/// This gives you a safe way to get a `&'static mut T` which is stored in `static` memory.
pub struct StoreButBorrowMut<T> {
    did_store: AtomicBool,
    data: UnsafeCell<MaybeUninit<T>>,
}

// Safety: `did_store` is `Sync` and `data` is only accessed once
unsafe impl<T> Sync for StoreButBorrowMut<T> {}

impl<T> StoreButBorrowMut<T> {
    pub const fn uninit() -> Self {
        Self {
            did_store: AtomicBool::new(false),
            data: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// You can only call this function once (and it checks in runtime). If this function was called for the first time, it returns `Ok` with a mut ref to the data you just stored. If this function was already back, it returns `Err` and gives you back the data.
    pub fn store_but_borrow_mut(&self, data: T) -> Result<&mut T, T> {
        if let Ok(_) =
            self.did_store
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        {
            // SAFETY: We have exclusive access to the data, and we do initialize it
            Ok(unsafe {
                let maybe_uninit = &mut *self.data.get();
                maybe_uninit.write(data);
                maybe_uninit.assume_init_mut()
            })
        } else {
            Err(data)
        }
    }
}
