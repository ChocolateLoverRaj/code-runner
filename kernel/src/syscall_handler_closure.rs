use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use common::syscall_uuids::{SYSCALL_EXISTS, SYSCALL_EXIT, SYSCALL_MAKE_ME_LOGGER};
use uuid::Uuid;
use x86_64::{
    registers::{
        control::Cr3,
        segmentation::{Segment, CS},
    },
    VirtAddr,
};

use crate::{
    context::{Context, FullContext},
    hhdm_offset::HhdmOffset,
    init_idt_and_gdt::get_priv_stack,
    limine_requests::HHDM_REQUEST,
    modules::syscall::syscall_handler_closure::PushedRegisters,
    pt_allocator_2::PHYS_MEM_TRACKER,
    run_tasks::run_tasks,
    tasks::{try_get_cpu_task_data, TaskType, TASKS},
};

pub trait Includes<K> {
    fn contains_key(&self, key: &K) -> bool;
}

impl<K: Ord, V> Includes<K> for BTreeMap<K, V> {
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

pub type SyscallHandler =
    dyn Fn([u64; 5], &mut PushedRegisters, &dyn Includes<Uuid>) -> ! + Send + Sync;

pub fn syscall_handler_closure(
) -> impl Fn(u64, u64, u64, u64, u64, u64, u64, &mut PushedRegisters) -> ! + Send + Sync + 'static {
    let syscall_handlers = {
        let mut syscall_handlers = BTreeMap::<Uuid, Box<SyscallHandler>>::new();
        // syscall_handlers.insert(
        //     SYSCALL_EXIT,
        //     Box::new(|_inputs, _pushed_registers, _| {
        //         log::info!("Syscall exit called");
        //         {
        //             let mut tasks = TASKS.lock();
        //             let (task_index, task) = tasks
        //                 .iter_mut()
        //                 .enumerate()
        //                 .find(|(_index, task)| match task.state {
        //                     TaskState::Running => true,
        //                     _ => false,
        //                 })
        //                 .unwrap();
        //             match &task.task_type {
        //                 TaskType::User(_data) => {
        //                     // FIXME: Cleanup Cr3 / page tables
        //                     // Kernel stack will be cleaned up by the `Drop` trait
        //                 }
        //             }
        //             tasks.remove(task_index);
        //         }
        //         log::info!("Running tasks");
        //         run_tasks()
        //     }),
        // );
        // syscall_handlers.insert(
        //     SYSCALL_EXISTS,
        //     Box::new(|inputs, pushed_registers, syscalls| {
        //         let return_value =
        //             match syscalls.contains_key(&Uuid::from_u64_pair(inputs[0], inputs[1])) {
        //                 true => 1,
        //                 false => 0,
        //             };
        //         let s = SyscallContext {
        //             r15: pushed_registers.r15,
        //             r14: pushed_registers.r14,
        //             r13: pushed_registers.r13,
        //             r12: pushed_registers.r12,
        //             rbx: pushed_registers.rbx,
        //             rbp: pushed_registers.rbp,
        //             r11: pushed_registers.r11,
        //             rcx: pushed_registers.rcx,
        //             rax: return_value,
        //             rsp: unsafe { THREAD_CONTROL_DATA.user_stack_pointer },
        //         };
        //         unsafe { GS::swap() };
        //         unsafe { s.restore() }
        //     }),
        // );
        // syscall_handlers.insert(
        //     SYSCALL_MAKE_ME_LOGGER,
        //     Box::new(get_syscall_make_me_logger_handler()),
        // );

        syscall_handlers
    };

    move |input0,
          input1,
          input2,
          input3,
          input4,
          input5,
          input6,
          pushed_registers: &mut PushedRegisters| {
        let syscall_uuid = Uuid::from_u64_pair(input0, input1);
        match syscall_handlers.get(&syscall_uuid) {
            None => {
                {
                    let mut cpu_task_data = try_get_cpu_task_data().unwrap().lock();
                    let process_id = cpu_task_data.current_task.unwrap();
                    log::warn!(
                        "Invalid syscall {:?} from process: {}. Terminating process.",
                        syscall_uuid,
                        process_id
                    );
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
                                phys_mem
                                    .set(segment.position..segment.position + segment.len, false);
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
            Some(syscall_handler) => syscall_handler(
                [input2, input3, input4, input5, input6],
                pushed_registers,
                &syscall_handlers,
            ),
        };
    }
}
