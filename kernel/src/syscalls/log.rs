use common::syscall_uuids::SyscallLog;
use x86_64::{
    structures::paging::{Mapper, Page, Size4KiB},
    VirtAddr,
};

use crate::{
    get_offset_page_table::get_offset_page_table, hhdm_offset::HhdmOffset,
    terminate_current_task::terminate_current_task,
};

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallLogHandler {
    pub hhdm_offset: HhdmOffset,
}
impl SyscallHandler2 for SyscallLogHandler {
    type Syscall = SyscallLog;

    fn handle_syscall(
        &self,
        input: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        // FIXME: Check pointer
        let message = if input.len() != 0 {
            let start = VirtAddr::from_ptr::<()>(input.into());
            let is_valid_slice = if start + input.len() <= VirtAddr::new(0xFFFF800000000000) {
                let mut page_range = Page::<Size4KiB>::containing_address(start)
                    ..=Page::<Size4KiB>::containing_address(start + input.len() - 1);
                let o = get_offset_page_table(self.hhdm_offset);
                loop {
                    match page_range.next() {
                        Some(page) => {
                            if o.translate_page(page).is_err() {
                                break false;
                            }
                        }
                        None => break true,
                    }
                }
            } else {
                false
            };
            if is_valid_slice {
                Ok(core::str::from_utf8(unsafe { input.to_slice() }).unwrap())
            } else {
                Err(())
            }
        } else {
            Ok(Default::default())
        };
        match message {
            Ok(message) => {
                log::info!("User space says {:?}", message);
            }
            Err(()) => {
                log::warn!("Log syscalled with invalid slice. Terminating process.");
                terminate_current_task()
            }
        }
    }
}
