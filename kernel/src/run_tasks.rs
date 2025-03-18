use x86_64::{registers::rflags::RFlags, VirtAddr};

use crate::{
    enter_user_mode::enter_user_mode,
    hlt_loop::hlt_loop,
    syscall_handler_closure::set_syscall_stack_pointer,
    tasks::{TaskState, TaskType, TASKS},
};

pub fn run_tasks() -> ! {
    let action: &dyn Fn() -> ! = {
        match TASKS.lock().first_mut() {
            Some(task) => {
                match task.state {
                    TaskState::ReadyToStart(state) => {
                        match &task.task_type {
                            TaskType::User(data) => {
                                // TODO: Change Cr3 if needed and flush TLB
                                task.state = TaskState::Running;
                                set_syscall_stack_pointer(VirtAddr::from_ptr(
                                    data.kernel_stack.as_ptr_range().end,
                                ));
                                &move || unsafe {
                                    enter_user_mode(
                                        state.instruction_pointer,
                                        state.stack_pointer,
                                        RFlags::INTERRUPT_FLAG, // FIXME: Set IOBP
                                                                // | RFlags::IOPL_HIGH
                                                                // | RFlags::IOPL_LOW,
                                    )
                                }
                            }
                        }
                    }
                    TaskState::Running => {
                        unreachable!("If the task is running, how is this code running? They both can't be running at the same time. Was the task state not updating from running to something else?");
                    }
                }
            }
            None => {
                log::warn!("No tasks to run. Halting.");
                &|| hlt_loop()
            }
        }
    };
    action()
}
