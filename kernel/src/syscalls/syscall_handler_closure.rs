use limine::response::FramebufferResponse;

use crate::hhdm_offset::HhdmOffset;

use super::{
    exists::SyscallExistsHandler,
    exit::SyscallExitHandler,
    listen_for_keyboard::SyscallListenForKeyboardHandler,
    log::SyscallLogHandler,
    raw_syscall_handler::SyscallHandlerClosure,
    screen::{SyscallReleaseScreenHandler, SyscallTakeScreenHandler},
    syscall_handlers::SyscallHandlers,
    take_io_port::SyscallTakeIoPortHandler,
    test::SyscallTestHandler,
    wait_until_event::SyscallWaitUntilEventHandler,
};

pub fn get_syscall_handlers(
    hhdm_offset: HhdmOffset,
    frame_buffer: Option<&'static FramebufferResponse>,
) -> impl SyscallHandlerClosure {
    let mut syscall_handlers = SyscallHandlers::default();
    syscall_handlers.insert_2(SyscallTestHandler);
    syscall_handlers.insert_2(SyscallExitHandler);
    syscall_handlers.insert_2(SyscallExistsHandler);
    syscall_handlers.insert_2(SyscallLogHandler { hhdm_offset });
    syscall_handlers.insert_2(SyscallTakeIoPortHandler);
    syscall_handlers.insert_2(SyscallListenForKeyboardHandler);
    syscall_handlers.insert_2(SyscallWaitUntilEventHandler);
    syscall_handlers.insert_2(SyscallTakeScreenHandler::new(hhdm_offset, frame_buffer));
    syscall_handlers.insert_2(SyscallReleaseScreenHandler::new(hhdm_offset, frame_buffer));
    syscall_handlers
}
