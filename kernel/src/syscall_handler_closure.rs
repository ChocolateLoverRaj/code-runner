use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use common::syscall_uuids::{SYSCALL_EXISTS, SYSCALL_EXIT};
use uuid::Uuid;
use x86_64::registers::segmentation::GS;

use crate::{
    context::{Context, SyscallContext},
    cpu_local_data::get_local,
    syscall_handler::{PushedRegisters, SyscallHandlerClosure},
    terminate_current_task::terminate_current_task,
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

pub fn get_syscall_handler_closure() -> SyscallHandlerClosure {
    let syscall_handlers = {
        let mut syscall_handlers = BTreeMap::<Uuid, Box<SyscallHandler>>::new();
        syscall_handlers.insert(
            SYSCALL_EXIT,
            Box::new(|_inputs, _pushed_registers, _| {
                log::info!("Syscall exit called");
                terminate_current_task()
            }),
        );
        // syscall_handlers.insert(
        //     SYSCALL_EXISTS,
        //     Box::new(|inputs, pushed_registers, syscalls| {
        //         log::info!("Syscall exists called");
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
        //             rsp: unsafe { &mut *get_local().get() }.user_stack_pointer,
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

    let closure = Box::new(
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
                    log::warn!("Invalid syscall {:?}. Terminating process.", syscall_uuid);
                    terminate_current_task()
                }
                Some(syscall_handler) => syscall_handler(
                    [input2, input3, input4, input5, input6],
                    pushed_registers,
                    &syscall_handlers,
                ),
            };
        },
    );
    // Safety: We do what it tells us to do
    unsafe { SyscallHandlerClosure::new(closure) }
}
