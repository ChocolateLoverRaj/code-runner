use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{
    syscall_pointer::SyscallPointer, syscall_slice::SyscallSlice,
    syscall_start_recording_keyboard::SyscallStartRecordingKeyboardInput,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub enum Syscall {
    Print(SyscallSlice),
    TakeFrameBuffer(SyscallPointer),
    Exit,
    StartRecordingKeyboard(SyscallStartRecordingKeyboardInput),
    PollKeyboard(SyscallSlice),
    /// Change the **total** number of allocated pages (the kernel increases / decreased depending on the current number and specified number)
    AllocatePages(u64),
    SetKeyboardInterruptHandler(Option<SyscallPointer>),
    /// Do not return from the keyboard interrupt handler. Instead, call this syscall at the end of ur fn.
    DoneWithInterruptHandler,
    DisableAndDeferMyInterrupts,
    EnableAndCatchUpOnMyInterrupts,
    EnableMyInterruptsAndWaitUntilOneHappens,
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
        let deserialized_syscall = Syscall::deserialize_from_input(serialized_syscall).unwrap();
        assert_eq!(deserialized_syscall, syscall);
    }
}
