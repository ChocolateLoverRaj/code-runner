use core::slice;

use bootloader_api::info::FrameBufferInfo;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{syscall_pointer::SyscallPointer, syscall_slice::SyscallSlice};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub enum Syscall {
    Print(SyscallSlice),
    TakeFrameBuffer(SyscallPointer),
}

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

impl TakeFrameBufferOutput {
    pub fn to_syscall_output(&self) -> postcard::Result<u64> {
        let mut output = [u8::default(); size_of::<u64>()];
        postcard::to_slice(&self, &mut output)?;
        Ok(u64::from_ne_bytes(output))
    }

    pub fn from_syscall_output(syscall_output: u64) -> postcard::Result<Self> {
        postcard::from_bytes(&syscall_output.to_ne_bytes())
    }
}

impl Syscall {
    pub fn serialize_to_input(&self) -> postcard::Result<[u64; 7]> {
        let mut arr: [u64; 7] = Default::default();
        postcard::to_slice(self, bytemuck::cast_slice_mut(&mut arr)).unwrap();
        Ok(arr)
    }

    pub fn deserialize_from_input(syscall: [u64; 7]) -> postcard::Result<Self> {
        let (syscall, _) = postcard::take_from_bytes(bytemuck::cast_slice(&syscall))?;
        Ok(syscall)
    }
}

#[cfg(test)]
mod test {
    use postcard::experimental::max_size::MaxSize;

    use super::*;

    #[test]
    fn fits_in_input() {
        assert_eq!(Syscall::POSTCARD_MAX_SIZE <= size_of::<u64>() * 7, true)
    }
}

#[cfg(test)]
mod test2 {
    use super::*;

    #[test]
    fn serialize_and_deserialize() {
        let syscall = Syscall::Print(SyscallSlice {
            pointer: 0x1000,
            len: 0xa,
        });
        let serialized_syscall = syscall.serialize_to_input().unwrap();
        let deserialiezd_syscall = Syscall::deserialize_from_input(serialized_syscall).unwrap();
        assert_eq!(deserialiezd_syscall, syscall);
    }
}
