use x86_64::registers::{
    control::Cr3,
    segmentation::{Segment, CS},
};

use crate::{
    context::{Context, FullContext},
    cpu_local_data::get_local,
    init_idt_and_gdt::get_priv_stack,
    physical_memory::{PhysicalMemoryState, UsedBy, PHYSICAL_MEMORY},
    run_tasks::run_tasks,
    tasks::{TaskType, TASKS},
};

/// Terminate the current task, switches to a different stack, and then runs other tasks
pub fn terminate_current_task() -> ! {
    {
        let mut cpu_task_data = get_local().unwrap().task_data.lock();
        let process_id = cpu_task_data.current_task.take().unwrap();
        // Switch Cr3 back to the kernel's Cr3 cuz we will be "deleting" the process's Cr3
        let mut tasks = TASKS.try_get().unwrap().lock();
        {
            let cr3_flags = Cr3::read().1;
            unsafe { Cr3::write(tasks.kernel_cr3, cr3_flags) };
        }
        let (task_index, task) = tasks
            .tasks
            .iter_mut()
            .enumerate()
            .find(|(_index, task)| task.id == process_id)
            .unwrap();
        // Clean up all phys frames
        // TODO: Maybe find out how to do this without cloning
        let mut physical_memory = PHYSICAL_MEMORY.try_get().unwrap().lock();
        physical_memory
            .clone()
            .into_iter()
            .for_each(|(range, state)| {
                if let PhysicalMemoryState::Used(UsedBy::UserSpace(task_id)) = state {
                    if task.id == task_id {
                        physical_memory.insert(range, PhysicalMemoryState::Available);
                    }
                }
            });

        match tasks.tasks.remove(task_index).task_type {
            TaskType::User(user_task_data) => {
                cpu_task_data.stack_to_delete = Some(user_task_data.kernel_stack);
            }
        }
    }
    let context = FullContext {
        rsp: get_priv_stack().top().as_u64(),
        rip: run_tasks as *const () as u64,
        cs: CS::get_reg().0 as u64,
        ..Default::default()
    };
    unsafe { context.restore() }
}
