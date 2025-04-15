use x86_64::{
    instructions::{interrupts, tlb::flush_all},
    registers::{control::Cr3, rflags::RFlags},
};

use crate::{
    cpu_local_data,
    get_offset_page_table::get_offset_page_table_with_l4,
    hlt_loop::hlt_loop,
    init_idt_and_gdt::get_iobp,
    io_permission_bitmap::IoPermissionBitmap,
    limine_requests::HHDM_REQUEST,
    modules::syscall::enter_user_mode::{enter_user_mode, EnterUserModeInput},
    tasks::{TaskState, TaskType, TASKS},
};

/// This function can be called through Rust code, or it can be entered through a iretq instruction
/// The iretq instruction method is for when this function needs to be entered while also switching stacks
pub extern "sysv64" fn run_tasks() -> ! {
    let hhdm_offset = (&HHDM_REQUEST).try_into().unwrap();
    enum Action {
        EnterUserMode(EnterUserModeInput),
        Halt,
    }
    let action: Action = {
        // Cleanup the previous stack
        let cpu_local_data = cpu_local_data::get_local().unwrap();
        let mut cpu_task_data = cpu_local_data.task_data.lock();
        cpu_task_data.stack_to_delete = None;
        let mut tasks = TASKS.try_get().unwrap().lock();
        // FIXME: Even though the allocator does invlpg, that only flushes the TLB for the CPU in which the allocator is running on.
        // It doesn't flush the TLB for other CPUs. Without this `flush_all` instruction, we can get page faults.
        flush_all();
        let kernel_cr3 = tasks.kernel_cr3;
        tasks
            .tasks
            .iter_mut()
            .find_map(|task| {
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
                                let kernel_stack_pointer = data.kernel_stack.top().as_u64();
                                unsafe {
                                    cpu_local_data
                                        .kernel_stack_pointer
                                        .get()
                                        .write(kernel_stack_pointer)
                                };
                                **get_iobp().lock() = IoPermissionBitmap::new_deny_all();
                                cpu_task_data.current_task = Some(task.id);
                                Some(Action::EnterUserMode(EnterUserModeInput {
                                    initialized_syscalls: *cpu_local_data
                                        .initialized_syscalls
                                        .try_get()
                                        .expect("Syscalls not initialized on this CPU"),
                                    code: state.instruction_pointer,
                                    stack_end: state.stack_pointer,
                                    rflags: RFlags::INTERRUPT_FLAG,
                                }))
                            }
                        }
                    }
                    TaskState::Running => None,
                    TaskState::WaitingUntilEvent(_) => None,
                }
            })
            .unwrap_or(Action::Halt)
    };
    match action {
        Action::EnterUserMode(input) => unsafe { enter_user_mode(input) },
        Action::Halt => {
            log::debug!("No tasks to run. Halting.");
            interrupts::enable();
            hlt_loop();
        }
    }
}
