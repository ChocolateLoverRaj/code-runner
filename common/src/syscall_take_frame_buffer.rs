use core::slice;

use bootloader_api::info::FrameBufferInfo;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::syscall_output::SyscallOutput;

/// Not `Copy` or `Clone` becuase it would allow multiple mutable references to the same memory
#[derive(Debug)]
pub struct TakeFrameBufferOutputData {
    buffer_start: u64,
    info: FrameBufferInfo,
}

impl TakeFrameBufferOutputData {
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.buffer_start as *mut _, self.info.byte_len) }
    }

    pub fn info(&self) -> FrameBufferInfo {
        self.info
    }
}

impl TakeFrameBufferOutputData {
    pub fn new(buffer_start: u64, info: FrameBufferInfo) -> Self {
        Self { buffer_start, info }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub enum TakeFrameBufferError {
    PointerIsNull,
    PointerNotAligned,
    /// The user space program is not allowed to access the pointer it provided
    PointerNotAllowed,
    NoFrameBuffer,
    /// There could be other MMIO in the same frames as the frame buffer so it would be unsecure to give access to all of the frame buffer's memory-mapped frames.
    CannotSecurelyGiveAccess,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub struct TakeFrameBufferOutput(pub Result<(), TakeFrameBufferError>);

impl SyscallOutput for TakeFrameBufferOutput {}
