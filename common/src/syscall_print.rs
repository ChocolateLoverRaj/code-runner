use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::syscall_output::SyscallOutput;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub enum SyscallPrintError {
    PointerIsNull,
    PointerNotAligned,
    /// The user space program is not allowed to access the pointer it provided
    PointerNotAllowed,
    /// The string is not valid utf8
    InvalidString,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub struct SyscallPrintOutput(pub Result<(), SyscallPrintError>);

impl SyscallOutput for SyscallPrintOutput {}
