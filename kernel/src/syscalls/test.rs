use common::syscall_uuids::SyscallTest;

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallTestHandler;
impl SyscallHandler2 for SyscallTestHandler {
    type Syscall = SyscallTest;

    fn handle_syscall(
        &self,
        input: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        if input == SyscallTest::TEST_INPUT {
            log::info!("Test syscall with expected input: {:?}", input);
        } else {
            log::warn!("Test syscall had unexpected input: {:?}", input);
        }
        SyscallTest::TEST_OUTPUT
    }
}
