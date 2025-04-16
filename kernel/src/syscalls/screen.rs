use core::ops::Deref;

use common::syscall_uuids::{ScreenInfo, Syscall, SyscallTakeScreen, SyscallTakeScreenError};
use limine::{framebuffer::Framebuffer, response::FramebufferResponse};

use crate::{
    cpu_local_data::get_local,
    logger_3,
    screen_lock::{WhoIsUsingScreen, SCREEN_LOCK},
    tasks::{TaskType, TASKS},
    terminate_current_task::terminate_current_task,
};

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallTakeScreenHandler {
    frame_buffer: Option<Framebuffer<'static>>,
}
impl SyscallTakeScreenHandler {
    pub fn new(frame_buffer: Option<&'static FramebufferResponse>) -> Self {
        Self {
            frame_buffer: frame_buffer.and_then(|f| f.framebuffers().next()),
        }
    }
}
impl SyscallHandler2 for SyscallTakeScreenHandler {
    type Syscall = SyscallTakeScreen;

    fn handle_syscall(
        &self,
        input: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        enum Action {
            Return(<SyscallTakeScreen as Syscall>::Output),
            Terminate,
        }
        let action = {
            let task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
            let mut tasks = TASKS.try_get().unwrap().lock();
            let task = tasks.tasks.iter().find(|task| task.id == task_id).unwrap();
            match &task.task_type {
                TaskType::User(data) => {
                    if data.permissions.screen {
                        if let Some(frame_buffer) = &self.frame_buffer {
                            let mut screen_lock = SCREEN_LOCK.lock();
                            match &*screen_lock {
                                Some(WhoIsUsingScreen::KernelLogger) => {
                                    logger_3::stop_using_frame_buffer();
                                    *screen_lock = Some(WhoIsUsingScreen::Task(task_id));
                                    todo!("Map frame_buffer to task's address space, and make it accessible to user mode")
                                    // Action::Return(Ok(ScreenInfo {
                                    //     address: frame_buffer.addr() as usize,
                                    // }))
                                }
                                Some(WhoIsUsingScreen::Task(_)) => {
                                    Action::Return(Err(SyscallTakeScreenError::InUse))
                                }
                                None => {
                                    todo!()
                                }
                            }
                        } else {
                            Action::Return(Err(SyscallTakeScreenError::NoScreenAvailable))
                        }
                    } else {
                        log::warn!("Task {} tried to take screen when it doesn't have permission. Terminating.", task_id);
                        Action::Terminate
                    }
                }
            }
        };
        match action {
            Action::Return(r) => r,
            Action::Terminate => terminate_current_task(),
        }
    }
}
