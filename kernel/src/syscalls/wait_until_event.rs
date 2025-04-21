use core::mem::MaybeUninit;

use common::syscall_uuids::{EventId, Syscall, SyscallWaitUntilEvent};
use x86_64::{structures::paging::PageTableFlags, VirtAddr};

use crate::{
    check_pointer::check_virt_addr_range,
    cpu_local_data::get_local,
    get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset,
    return_wait_until_event::handle_pending_events,
    run_tasks::run_tasks,
    tasks::{SavedSyscallState, TaskState, WaitingUntilEventData, TASKS},
    terminate_current_task::terminate_current_task,
};

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallWaitUntilEventHandler {
    hhdm_offset: HhdmOffset,
}
impl SyscallWaitUntilEventHandler {
    pub fn new(hhdm_offset: HhdmOffset) -> Self {
        Self { hhdm_offset }
    }
}
impl SyscallHandler2 for SyscallWaitUntilEventHandler {
    type Syscall = SyscallWaitUntilEvent;

    fn handle_syscall(
        &self,
        input: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        enum Action {
            Return(<SyscallWaitUntilEvent as Syscall>::Output),
            RunTasks,
            TerminateTask,
        }
        let action = {
            let mut tasks = TASKS.lock();
            let cpu_local_data = get_local().unwrap();
            let mut task_data = cpu_local_data.task_data.lock();
            let task_id = task_data.current_task.unwrap();
            if input.len() > 0 {
                let output_virt_addr_range = {
                    let start = VirtAddr::from_ptr::<()>(input.into());
                    start..start + input.len() * size_of::<MaybeUninit<EventId>>() as u64
                };
                log::info!("Output virt addr range: {:?}", output_virt_addr_range);
                match check_virt_addr_range(
                    output_virt_addr_range,
                    &get_offset_page_table(self.hhdm_offset),
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::USER_ACCESSIBLE
                        | PageTableFlags::NO_EXECUTE,
                ) {
                    Ok(()) => match unsafe { handle_pending_events(task_id, input) } {
                        Some(r) => Action::Return(r),
                        None => {
                            let current_task = tasks.get_mut(&task_id).unwrap();
                            log::debug!("Saved task state since it's waiting for event.");
                            current_task.state =
                                TaskState::WaitingUntilEvent(WaitingUntilEventData {
                                    state: SavedSyscallState {
                                        pushed_registers: *pushed_registers,
                                        stack_pointer: unsafe {
                                            cpu_local_data.user_stack_pointer.get().read()
                                        },
                                    },
                                    input,
                                });
                            task_data.current_task = None;
                            Action::RunTasks
                        }
                    },
                    Err(e) => {
                        log::warn!("Task {:?} tried to pass an invalid slice: {:?} as a wait until event argument. Error: {:#?}. Terminating.", task_id, input, e);
                        Action::TerminateTask
                    }
                }
            } else {
                // Ther is no reason to ever wait for 0 events
                log::warn!(
                    "Task {:?} tried to wait for 0 events. Terminating task.",
                    task_id
                );
                Action::TerminateTask
            }
        };
        match action {
            Action::Return(r) => r,
            Action::RunTasks => run_tasks(),
            Action::TerminateTask => terminate_current_task(),
        }
    }
}
