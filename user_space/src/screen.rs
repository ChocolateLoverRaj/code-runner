use common::syscall_uuids::{ScreenInfo, SyscallTakeScreenError, SyscallTakeScreenOutput};

use crate::syscall::{syscall_release_screen, syscall_take_screen};

pub struct Screen {
    o: SyscallTakeScreenOutput,
}

impl Screen {
    pub fn take() -> Result<Self, SyscallTakeScreenError> {
        Ok(Self {
            o: syscall_take_screen()?,
        })
    }

    pub fn frame_buffer_mut(&mut self) -> &mut [u8] {
        // Safety: The kernel mapped this process's memory to the frame buffer and nothing else is referencing it
        unsafe {
            core::slice::from_raw_parts_mut(
                self.o.address as *mut u8,
                (self.o.info.pitch * self.o.info.height) as usize,
            )
        }
    }

    pub fn info(&self) -> &ScreenInfo {
        &self.o.info
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        syscall_release_screen();
    }
}
