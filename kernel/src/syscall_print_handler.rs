use core::str;

use common::{
    mem::KERNEL_VIRT_MEM_START,
    syscall_output::SyscallOutput,
    syscall_print::{SyscallPrintError, SyscallPrintOutput},
    syscall_slice::SyscallSlice,
};
use x86_64::VirtAddr;

pub fn syscall_print_handler(message: SyscallSlice) -> u64 {
    let output = SyscallPrintOutput({
        let pointer: *const u8 = message.into();
        if pointer.is_null() {
            Err(SyscallPrintError::PointerIsNull)
        } else if !pointer.is_aligned() {
            Err(SyscallPrintError::PointerNotAligned)
        } else if VirtAddr::from_ptr(pointer.wrapping_add(message.len() as usize))
            > VirtAddr::new_truncate(KERNEL_VIRT_MEM_START)
        {
            Err(SyscallPrintError::PointerNotAllowed)
        } else {
            match str::from_utf8(unsafe { message.to_slice() }) {
                Ok(message) => {
                    log::info!("[U] {:?}", message);
                    Ok(())
                }
                Err(_e) => Err(SyscallPrintError::InvalidString),
            }
        }
    });
    output.to_syscall_output().unwrap()
}
