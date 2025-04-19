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
    tasks::{KERNEL_CR3, TASKS, TASK_QUEUE},
};

/// Terminate the current task, switches to a different stack, and then runs other tasks
pub fn terminate_current_task() -> ! {
    {
        let mut cpu_task_data = get_local().unwrap().task_data.lock();
        let task_id = cpu_task_data.current_task.take().unwrap();
        // Switch Cr3 back to the kernel's Cr3 cuz we will be "deleting" the process's Cr3
        let mut tasks = TASKS.lock();
        {
            let cr3_flags = Cr3::read().1;
            unsafe { Cr3::write(*KERNEL_CR3.try_get().unwrap(), cr3_flags) };
        }
        let task = tasks.remove(&task_id).unwrap();
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
        cpu_task_data.stack_to_delete = Some(task.kernel_stack);
        let mut queue = TASK_QUEUE.lock();
        let task_index = queue
            .iter()
            .enumerate()
            .find_map(|(index, id)| if *id == task_id { Some(index) } else { None })
            .unwrap();
        queue.remove(task_index);
    }
    let context = FullContext {
        rsp: get_priv_stack().top().as_u64(),
        rip: run_tasks as *const () as u64,
        cs: CS::get_reg().0 as u64,
        ..Default::default()
    };
    unsafe { context.restore() }
}
