use alloc::boxed::Box;
use util::init_later::{InitLater, TryGetError, TryInitError};

use crate::{cpu_local_data::get_local, limine_requests::MP_REQUEST};

fn get_cpu_count() -> usize {
    unsafe { &MP_REQUEST }.get_response().unwrap().cpus().len()
}

pub struct CpuLocal<T> {
    data: InitLater<Box<[T]>>,
}

impl<T> CpuLocal<T> {
    pub const fn uninit() -> Self {
        Self {
            data: InitLater::uninit(),
        }
    }

    /// Allocates
    pub fn try_init<F: FnMut() -> T>(&self, mut init: F) -> Result<(), TryInitError> {
        let cpu_count = get_cpu_count();
        self.data
            .try_init((0..cpu_count).map(|_| init()).collect())?;
        Ok(())
    }

    pub fn try_get(&self) -> Result<&T, TryGetError> {
        let b = self.data.try_get()?;
        Ok(&b[unsafe { get_local().get().read().cpu_id }])
    }
}
