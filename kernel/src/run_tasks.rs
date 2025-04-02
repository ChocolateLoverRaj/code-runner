use x86_64::{
    registers::{control::Cr3, rflags::RFlags},
    VirtAddr,
};

use crate::{
    cpu_local_data::set_syscall_stack_pointer,
    hlt_loop::hlt_loop,
    init_idt_and_gdt::get_iobp,
    limine_requests::HHDM_REQUEST,
    modules::syscall::enter_user_mode::{enter_user_mode, EnterUserModeInput},
    pt_allocator_2::get_offset_page_table::get_offset_page_table_with_l4,
    tasks::{try_get_cpu_task_data, TaskState, TaskType, TASKS},
};

/// This function can be called through Rust code, or it can be entered through a iretq instruction
/// The iretq instruction method is for when this function needs to be entered while also switching stacks
pub extern "sysv64" fn run_tasks() -> ! {
    // Cleanup the previous stack
    try_get_cpu_task_data().unwrap().lock().stack_to_delete = None;

    let hhdm_offset = (&HHDM_REQUEST).try_into().unwrap();
    enum Action {
        EnterUserMode(EnterUserModeInput),
        Halt,
    }
    let action: Action = {
        let mut tasks = TASKS.try_get().unwrap().lock();
        let kernel_cr3 = tasks.kernel_cr3;
        match tasks.tasks.first_mut() {
            Some(task) => {
                match task.state {
                    TaskState::ReadyToStart(state) => {
                        match &task.task_type {
                            TaskType::User(data) => {
                                // Map all upper half L3 pages to the L4 page table
                                let mut task_page_table =
                                    get_offset_page_table_with_l4(data.cr3, hhdm_offset);
                                let kernel_page_table =
                                    get_offset_page_table_with_l4(kernel_cr3, hhdm_offset);
                                for i in 256..512 {
                                    task_page_table.level_4_table_mut()[i] =
                                        kernel_page_table.level_4_table()[i].clone();
                                }
                                let cr3_flags = Cr3::read().1;
                                unsafe { Cr3::write(data.cr3, cr3_flags) };

                                task.state = TaskState::Running;
                                set_syscall_stack_pointer(VirtAddr::from_ptr(
                                    data.kernel_stack.as_ptr_range().end,
                                ));
                                **get_iobp().lock() = data.iopb;
                                try_get_cpu_task_data().unwrap().lock().current_task =
                                    Some(task.id);
                                Action::EnterUserMode(EnterUserModeInput {
                                    initialized_syscalls: tasks.initialized_syscalls,
                                    code: state.instruction_pointer,
                                    stack_end: state.stack_pointer,
                                    rflags: RFlags::INTERRUPT_FLAG,
                                })
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
                Action::Halt
            }
        }
    };
    match action {
        Action::EnterUserMode(input) => unsafe { enter_user_mode(input) },
        Action::Halt => {
            hlt_loop();
        }
    }
}
