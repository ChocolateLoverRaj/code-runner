use core::mem::MaybeUninit;

use common::syscall_uuids::{EventId, Syscall, SyscallWaitUntilEvent};
use x86_64::{structures::paging::PageTableFlags, VirtAddr};

use crate::{
    check_pointer::check_virt_addr_range,
    cpu_local_data::get_local,
    get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset,
    run_tasks::run_tasks,
    tasks::{SavedSyscallState, TaskState, WaitingUntilEventData, KEYBOARD_LISTENER, TASKS},
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
            let output_virt_addr_range = {
                let start = VirtAddr::from_ptr::<()>(input.into());
                start..start + input.len()
            };
            match check_virt_addr_range(
                output_virt_addr_range,
                &get_offset_page_table(self.hhdm_offset),
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::NO_EXECUTE,
            ) {
                Ok(()) => match &mut *KEYBOARD_LISTENER.lock() {
                    Some(keyboard_event_listener) => {
                        if keyboard_event_listener.task_id == task_id {
                            if keyboard_event_listener.pending_interrupt_received {
                                keyboard_event_listener.pending_interrupt_received = false;
                                let events_that_happened =
                                    unsafe { input.to_slice_mut::<MaybeUninit<EventId>>() };
                                if let Some(first_slot) = events_that_happened.first_mut() {
                                    first_slot.write(EventId::Keyboard);
                                }
                                Action::Return(1)
                            } else {
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
                        } else {
                            Action::Return(0)
                        }
                    }
                    None => Action::Return(0),
                },
                Err(e) => {
                    log::warn!("Task {:?} tried to pass an invalid slice: {:?} as a wait until event argument. Error: {:#?}. Terminating.", task_id, input, e);
                    Action::TerminateTask
                }
            }
        };
        match action {
            Action::Return(r) => r,
            Action::RunTasks => run_tasks(),
            Action::TerminateTask => terminate_current_task(),
        }
    }
}
