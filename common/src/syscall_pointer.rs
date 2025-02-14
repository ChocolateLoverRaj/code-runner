use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub struct SyscallPointer(u64);

impl<T> From<*const T> for SyscallPointer {
    fn from(value: *const T) -> Self {
        Self(value as u64)
    }
}

impl<T> From<*mut T> for SyscallPointer {
    fn from(value: *mut T) -> Self {
        Self(value as u64)
    }
}

impl<T> From<SyscallPointer> for *const T {
    fn from(value: SyscallPointer) -> Self {
        value.0 as *const T
    }
}

impl<T> From<SyscallPointer> for *mut T {
    fn from(value: SyscallPointer) -> Self {
        value.0 as *mut T
    }
}
