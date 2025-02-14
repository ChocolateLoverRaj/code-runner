use core::slice;

use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub struct SyscallSlice {
    pub(crate) pointer: u64,
    /// The len of whatever type is being represented, not necessarily the number of `u8`s
    pub(crate) len: u64,
}

impl SyscallSlice {
    pub fn from_slice<T>(slice: &[T]) -> Self {
        Self {
            pointer: slice.as_ptr() as u64,
            len: slice.len() as u64,
        }
    }

    /// # Safety
    /// See `core::slice::from_raw_parts`
    pub unsafe fn to_slice<'a, T>(&self) -> &'a [T] {
        unsafe { slice::from_raw_parts(self.pointer as *const _, self.len as usize) }
    }
}

impl<T> From<&[T]> for SyscallSlice {
    fn from(value: &[T]) -> Self {
        Self {
            pointer: value.as_ptr() as u64,
            len: value.len() as u64,
        }
    }
}

impl<T> From<SyscallSlice> for *mut T {
    fn from(value: SyscallSlice) -> Self {
        value.pointer as *mut T
    }
}
