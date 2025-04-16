use core::fmt::Debug;

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use common::syscall_uuids::{from_input_without_uuid, get_uuid, serialize_output, Syscall};
use uuid::Uuid;
use x86_64::registers::segmentation::GS;

use crate::{
    context::{Context, SyscallContext},
    terminate_current_task::terminate_current_task,
};

use super::raw_syscall_handler::{PushedRegisters, SyscallHandlerClosure};

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
        let input = from_input_without_uuid::<T>(input).unwrap();
        let output = f(input, pushed_registers, syscalls);
        serialize_output::<T>(&output).unwrap()
    }
}

pub trait SyscallHandler2: Send + Sync {
    type Syscall: Syscall;

    fn handle_syscall(
        &self,
        input: <Self::Syscall as Syscall>::Input,
        pushed_registers: &mut PushedRegisters,
        syscalls: &dyn Includes<Uuid>,
    ) -> <Self::Syscall as Syscall>::Output;
}

#[derive(Default)]
pub struct SyscallHandlers {
    handlers: BTreeMap<Uuid, Box<SyscallHandler>>,
}
impl SyscallHandlers {
    pub fn insert<T: Syscall + 'static>(
        &mut self,
        handler: impl Fn(T::Input, &PushedRegisters, &dyn Includes<Uuid>) -> T::Output
            + Send
            + Sync
            + 'static,
    ) {
        self.handlers
            .insert(T::UUID, Box::new(make_syscall_handler::<T>(handler)));
    }

    pub fn insert_2<T: SyscallHandler2 + 'static>(&mut self, handler: T) {
        self.handlers.insert(
            T::Syscall::UUID,
            Box::new(move |input, pushed_registers, syscalls| {
                let input = from_input_without_uuid::<T::Syscall>(input).unwrap();
                let output = handler.handle_syscall(input, pushed_registers, syscalls);
                serialize_output::<T::Syscall>(&output).unwrap()
            }),
        );
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
                let s = SyscallContext::from_syscall_output(pushed_registers, output);
                unsafe { GS::swap() };
                unsafe { s.restore() }
            }
        }
    }
}
