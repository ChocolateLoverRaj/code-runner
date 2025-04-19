use common::syscall_uuids::SyscallExists;

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallExistsHandler;
impl SyscallHandler2 for SyscallExistsHandler {
    type Syscall = SyscallExists;

    fn handle_syscall(
        &self,
        uuid: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        syscall_handlers: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        syscall_handlers.contains_key(&uuid)
    }
}
