use common::syscall_uuids::SyscallExit;

use crate::terminate_current_task::terminate_current_task;

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallExitHandler;
impl SyscallHandler2 for SyscallExitHandler {
    type Syscall = SyscallExit;

    fn handle_syscall(
        &self,
        _input: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        log::info!("Syscall exit called");
        terminate_current_task()
    }
}
