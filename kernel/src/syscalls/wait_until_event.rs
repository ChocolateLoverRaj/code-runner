use common::syscall_uuids::{Syscall, SyscallWaitUntilEvent};

use crate::{
    cpu_local_data::get_local,
    run_tasks::run_tasks,
    tasks::{SavedSyscallState, TaskState, KEYBOARD_LISTENER, TASKS},
};

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallWaitUntilEventHandler;
impl SyscallHandler2 for SyscallWaitUntilEventHandler {
    type Syscall = SyscallWaitUntilEvent;

    fn handle_syscall(
        &self,
        _input: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        enum Action {
            Return(<SyscallWaitUntilEvent as Syscall>::Output),
            RunTasks,
        }
        let action = {
            let mut tasks = TASKS.lock();
            let cpu_local_data = get_local().unwrap();
            let mut task_data = cpu_local_data.task_data.lock();
            let task_id = task_data.current_task.unwrap();
            match &mut *KEYBOARD_LISTENER.lock() {
                Some(keyboard_event_listener) => {
                    if keyboard_event_listener.task_id == task_id {
                        if keyboard_event_listener.pending_interrupt_received {
                            keyboard_event_listener.pending_interrupt_received = false;
                            Action::Return(())
                        } else {
                            let current_task = tasks.get_mut(&task_id).unwrap();
                            log::debug!("Saved task state since it's waiting for event.");
                            current_task.state = TaskState::WaitingUntilEvent(SavedSyscallState {
                                pushed_registers: *pushed_registers,
                                stack_pointer: unsafe {
                                    cpu_local_data.user_stack_pointer.get().read()
                                },
                            });
                            task_data.current_task = None;
                            Action::RunTasks
                        }
                    } else {
                        Action::Return(())
                    }
                }
                None => Action::Return(()),
            }
        };
        match action {
            Action::Return(r) => r,
            Action::RunTasks => run_tasks(),
        }
    }
}
