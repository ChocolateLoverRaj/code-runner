use common::syscall_uuids::SyscallLog;
use x86_64::{structures::paging::PageTableFlags, VirtAddr};

use crate::{
    check_pointer::check_virt_addr_range, get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset, terminate_current_task::terminate_current_task,
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
        match check_virt_addr_range(
            {
                let start = VirtAddr::from_ptr::<()>(input.into());
                start..start + size_of::<u8>() as u64 * input.len()
            },
            &get_offset_page_table(self.hhdm_offset),
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE,
        ) {
            Ok(()) => {
                let message = unsafe { input.to_slice() };
                match core::str::from_utf8(message) {
                    Ok(message) => {
                        log::info!("User space says {:?}", message);
                    }
                    Err(_e) => {
                        log::info!("User space says (invalid UTF-8) {:?}", message);
                    }
                };
            }
            Err(e) => {
                log::warn!(
                    "Log syscalled with invalid slice. Error: {:#?}. Terminating process.",
                    e
                );
                terminate_current_task()
            }
        }
    }
}
