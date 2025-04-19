use x86_64::{
    instructions::interrupts,
    registers::{control::Cr3, rflags::RFlags, segmentation::GS},
};

use crate::{
    context::{Context, FullContext},
    cpu_local_data,
    get_offset_page_table::get_offset_page_table_with_l4,
    hlt_loop::hlt_loop,
    init_idt_and_gdt::get_iopb,
    io_permission_bitmap::IoPermissionBitmap,
    io_ports_lock::{IoPortUsedBy, IO_PORT_USAGE},
    limine_requests::HHDM_REQUEST,
    modules::syscall::enter_user_mode::{enter_user_mode, EnterUserModeInput},
    tasks::{TaskId, TaskState, KERNEL_CR3, TASKS, TASK_QUEUE},
};

/// This function can be called through Rust code, or it can be entered through a iretq instruction
/// The iretq instruction method is for when this function needs to be entered while also switching stacks
pub extern "sysv64" fn run_tasks() -> ! {
    let hhdm_offset = (&HHDM_REQUEST).try_into().unwrap();
    enum Action {
        RestoreContext(FullContext),
        EnterUserMode(EnterUserModeInput),
        Halt,
    }
    let action = {
        // Cleanup the previous stack
        let cpu_local_data = cpu_local_data::get_local().unwrap();
        let mut cpu_task_data = cpu_local_data.task_data.lock();
        cpu_task_data.stack_to_delete = None;
        let task_queue = TASK_QUEUE.lock();
        let mut tasks = TASKS.lock();
        task_queue
            .iter()
            .find_map(|task_id| {
                let task = tasks.get_mut(task_id).unwrap();
                match &task.state {
                    TaskState::ReadyToStart(state) => {
                        // Map all upper half L3 pages to the L4 page table
                        let mut task_page_table =
                            get_offset_page_table_with_l4(task.cr3, hhdm_offset);
                        let kernel_page_table = get_offset_page_table_with_l4(
                            *KERNEL_CR3.try_get().unwrap(),
                            hhdm_offset,
                        );
                        for i in 256..512 {
                            task_page_table.level_4_table_mut()[i] =
                                kernel_page_table.level_4_table()[i].clone();
                        }
                        let cr3_flags = Cr3::read().1;
                        unsafe { Cr3::write(task.cr3, cr3_flags) };
                        let enter_user_mode_input = EnterUserModeInput {
                            initialized_syscalls: *cpu_local_data
                                .initialized_syscalls
                                .try_get()
                                .expect("Syscalls not initialized on this CPU"),
                            code: state.instruction_pointer,
                            stack_end: state.stack_pointer,
                            rflags: RFlags::INTERRUPT_FLAG,
                        };
                        task.state = TaskState::Running;
                        let kernel_stack_pointer = task.kernel_stack.top().as_u64();
                        unsafe {
                            cpu_local_data
                                .kernel_stack_pointer
                                .get()
                                .write(kernel_stack_pointer)
                        };
                        update_iobp(*task_id);
                        cpu_task_data.current_task = Some(*task_id);
                        Some(Action::EnterUserMode(enter_user_mode_input))
                    }
                    TaskState::Interrupted(full_context) => {
                        let full_context = *full_context;
                        let cr3_flags = Cr3::read().1;
                        unsafe { Cr3::write(task.cr3, cr3_flags) };
                        update_iobp(*task_id);
                        task.state = TaskState::Running;
                        cpu_task_data.current_task = Some(*task_id);
                        log::debug!("Restoring context");
                        Some(Action::RestoreContext(full_context))
                    }
                    _ => None,
                }
            })
            .unwrap_or(Action::Halt)
    };
    match action {
        Action::RestoreContext(context) => unsafe {
            log::debug!(
                "Restoring full context: {:?}",
                RFlags::from_bits_truncate(context.rflags)
            );
            GS::swap();
            context.restore()
        },
        Action::EnterUserMode(input) => unsafe { enter_user_mode(input) },
        Action::Halt => {
            log::debug!("No tasks to run. Halting.");
            interrupts::enable();
            hlt_loop();
        }
    }
}

pub fn update_iobp(task_id: TaskId) {
    let mut iopb = get_iopb().lock();
    **iopb = IoPermissionBitmap::new_deny_all();
    IO_PORT_USAGE
        .lock()
        .iter()
        .for_each(|(port, used_by)| match used_by {
            IoPortUsedBy::User(used_by_task_id) => {
                if *used_by_task_id == task_id {
                    iopb.set_port_allowed(*port, true);
                }
            }
            IoPortUsedBy::Kernel => {}
        });
}
