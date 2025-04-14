use core::fmt::Debug;

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use common::syscall_uuids::{
    get_uuid, Syscall, SyscallExists, SyscallExit, SyscallLog, SyscallTest,
};
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
    dyn Fn(&[u64; 5], &mut PushedRegisters, &dyn Includes<Uuid>) -> [u64; 7] + Send + Sync;

fn make_syscall_handler<T: Syscall>(
    f: impl Fn(T::Input, &PushedRegisters, &dyn Includes<Uuid>) -> T::Output + Send + Sync,
) -> impl Fn(&[u64; 5], &mut PushedRegisters, &dyn Includes<Uuid>) -> [u64; 7] + Send + Sync {
    move |input, pushed_registers, syscalls| {
        let input = T::from_input_without_uuid(input).unwrap();
        let output = f(input, pushed_registers, syscalls);
        T::serialize_output(&output).unwrap()
    }
}

#[derive(Default)]
struct SyscallHandlers {
    handlers: BTreeMap<Uuid, Box<SyscallHandler>>,
}
impl SyscallHandlers {
    fn insert<T: Syscall + 'static>(
        &mut self,
        handler: impl Fn(T::Input, &PushedRegisters, &dyn Includes<Uuid>) -> T::Output
            + Send
            + Sync
            + 'static,
    ) {
        self.handlers
            .insert(T::UUID, Box::new(make_syscall_handler::<T>(handler)));
    }
}

impl Debug for SyscallHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(core::any::type_name::<Self>())
            .field("syscalls", &self.handlers.keys())
            .finish()
    }
}

// Safety: We do what it tells us to do
unsafe impl SyscallHandlerClosure for SyscallHandlers {
    fn handle_syscall(
        &self,
        input0: u64,
        input1: u64,
        input2: u64,
        input3: u64,
        input4: u64,
        input5: u64,
        input6: u64,
        pushed_registers: &mut PushedRegisters,
    ) -> ! {
        let (uuid, input) =
            get_uuid([input0, input1, input2, input3, input4, input5, input6]).unwrap();
        match self.handlers.get(&uuid) {
            None => {
                log::warn!("Invalid syscall {:?}. Terminating process.", uuid);
                terminate_current_task()
            }
            Some(syscall_handler) => {
                let output = syscall_handler(&input, pushed_registers, &self.handlers);
                let s = SyscallContext {
                    r15: pushed_registers.r15,
                    r14: pushed_registers.r14,
                    r13: pushed_registers.r13,
                    r12: pushed_registers.r12,
                    rbx: pushed_registers.rbx,
                    rbp: pushed_registers.rbp,
                    r11: pushed_registers.r11,
                    rcx: pushed_registers.rcx,
                    rdi: output[0],
                    rsi: output[1],
                    rdx: output[2],
                    r10: output[3],
                    r8: output[4],
                    r9: output[5],
                    rax: output[6],
                    rsp: unsafe { get_local().unwrap().user_stack_pointer.get().read() },
                };
                unsafe { GS::swap() };
                unsafe { s.restore() }
            }
        }
    }
}

pub fn get_syscall_handlers() -> impl SyscallHandlerClosure {
    let mut syscall_handlers = SyscallHandlers::default();
    syscall_handlers.insert::<SyscallTest>(|input, _, _| {
        if input == SyscallTest::TEST_INPUT {
            log::info!("Test syscall with expected input: {:?}", input);
        } else {
            log::warn!("Test syscall had unexpected input: {:?}", input);
        }
        SyscallTest::TEST_OUTPUT
    });
    syscall_handlers.insert::<SyscallExit>(|_, _, _| {
        log::info!("Syscall exit called");
        terminate_current_task()
    });
    syscall_handlers
        .insert::<SyscallExists>(|uuid, _, syscall_handlers| syscall_handlers.contains_key(&uuid));
    syscall_handlers.insert::<SyscallLog>(|message, _, _| {
        // FIXME: Check pointer
        let message = core::str::from_utf8(unsafe { message.to_slice() }).unwrap();
        log::info!("User space says {:?}", message);
    });
    syscall_handlers
}
