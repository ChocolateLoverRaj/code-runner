use x86_64::registers::{
    control::Cr3,
    segmentation::{Segment, CS},
};

use crate::{
    context::{Context, FullContext},
    init_idt_and_gdt::get_priv_stack,
    pt_allocator_2::PHYS_MEM_TRACKER,
    run_tasks::run_tasks,
    tasks::{try_get_cpu_task_data, TaskType, TASKS},
};

/// Terminate the current task, switches to a different stack, and then runs other tasks
pub fn terminate_current_task() -> ! {
    {
        let mut cpu_task_data = try_get_cpu_task_data().unwrap().lock();
        let process_id = cpu_task_data.current_task.unwrap();
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
        {
            let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
            task.owned_phys_mem
                .iter()
                .filter(|segment| segment.value)
                .for_each(|segment| {
                    phys_mem.set(segment.position..segment.position + segment.len, false);
                });
        }
        match tasks.tasks.remove(task_index).task_type {
            TaskType::User(user_task_data) => {
                cpu_task_data.stack_to_delete = Some(user_task_data.kernel_stack);
            }
        }
    }
    let context = FullContext {
        rsp: get_priv_stack().as_ptr_range().end as u64,
        rip: run_tasks as *const () as u64,
        cs: CS::get_reg().0 as u64,
        ..Default::default()
    };
    unsafe { context.restore() }
}
