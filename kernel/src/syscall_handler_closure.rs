use alloc::collections::btree_map::BTreeMap;
use common::syscall_uuids::{SYSCALL_EXISTS, SYSCALL_EXIT};
use uuid::Uuid;
use x86_64::{
    registers::{model_specific::KernelGsBase, segmentation::GS},
    VirtAddr,
};

use crate::{
    context::{Context, SyscallContext},
    modules::syscall::syscall_handler_closure::{PushedRegisters, THREAD_CONTROL_DATA},
};

trait Includes<K> {
    fn contains_key(&self, key: &K) -> bool;
}

impl<K: Ord, V> Includes<K> for BTreeMap<K, V> {
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

type SyscallHandler =
    dyn Fn([u64; 5], &mut PushedRegisters, &dyn Includes<Uuid>) -> ! + Send + Sync;

pub fn syscall_handler_closure(
) -> impl Fn(u64, u64, u64, u64, u64, u64, u64, &mut PushedRegisters) -> ! + Send + Sync + 'static {
    let syscall_handlers = {
        let mut syscall_handlers = BTreeMap::<Uuid, &'static SyscallHandler>::new();
        syscall_handlers.insert(SYSCALL_EXIT, &|inputs, pushed_registers, _| todo!("Exit"));
        syscall_handlers.insert(SYSCALL_EXISTS, &|inputs, pushed_registers, syscalls| {
            let return_value =
                match syscalls.contains_key(&Uuid::from_u64_pair(inputs[0], inputs[1])) {
                    true => 1,
                    false => 0,
                };
            let s = SyscallContext {
                r15: pushed_registers.r15,
                r14: pushed_registers.r14,
                r13: pushed_registers.r13,
                r12: pushed_registers.r12,
                rbx: pushed_registers.rbx,
                rbp: pushed_registers.rbp,
                r11: pushed_registers.r11,
                rcx: pushed_registers.rcx,
                rax: return_value,
                rsp: unsafe { THREAD_CONTROL_DATA.user_stack_pointer },
            };
            unsafe { GS::swap() };
            unsafe { s.restore() }
        });
        syscall_handlers
    };

    KernelGsBase::write(VirtAddr::from_ptr(unsafe {
        &THREAD_CONTROL_DATA as *const _
    }));

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
                panic!("Invalid syscall");
            }
            Some(syscall_handler) => syscall_handler(
                [input2, input3, input4, input5, input6],
                pushed_registers,
                &syscall_handlers,
            ),
        };
    }
}

pub fn set_syscall_stack_pointer(syscall_stack_pointer: VirtAddr) {
    // This is needed to access `gs:` in asm
    unsafe { THREAD_CONTROL_DATA.kernel_stack_pointer = syscall_stack_pointer.as_u64() };
}
