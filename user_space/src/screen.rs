use common::{screen_info::ScreenInfoWithAddress, syscall_uuids::SyscallTakeScreenError};

use crate::syscall::{syscall_release_screen, syscall_take_screen};

pub struct Screen {
    screen: ScreenInfoWithAddress,
}

impl Screen {
    pub fn take() -> Result<Self, SyscallTakeScreenError> {
        Ok(Self {
            screen: syscall_take_screen()?,
        })
    }

    pub fn frame_buffer_mut(&mut self) -> &mut [u8] {
        // Safety: The kernel mapped this process's memory to the frame buffer and nothing else is referencing it
        unsafe {
            core::slice::from_raw_parts_mut(
                self.screen.address as *mut u8,
                (self.screen.info.pitch * self.screen.info.height) as usize,
            )
        }
    }

    pub fn screen_mut(&mut self) -> &mut ScreenInfoWithAddress {
        &mut self.screen
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        syscall_release_screen().unwrap();
    }
}
