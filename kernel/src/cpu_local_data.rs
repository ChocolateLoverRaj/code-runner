use core::cell::SyncUnsafeCell;

use alloc::boxed::Box;
use limine::response::MpResponse;
use util::init_later::InitLater;
use x86_64::{registers::model_specific::GsBase, VirtAddr};

/// This is what we set `GS.Base` to point to
#[derive(Debug)]
#[repr(packed)]
pub struct CpuLocalData {
    pub user_stack_pointer: u64,
    pub kernel_stack_pointer: u64,
    pub cpu_id: usize,
}

static CPU_LOCAL_DATA: InitLater<Box<[SyncUnsafeCell<CpuLocalData>]>> = InitLater::uninit();

pub fn init(mp_response: &MpResponse) {
    CPU_LOCAL_DATA
        .try_init(
            mp_response
                .cpus()
                .iter()
                .map(|cpu| {
                    SyncUnsafeCell::new(CpuLocalData {
                        user_stack_pointer: 0,
                        kernel_stack_pointer: 0,
                        cpu_id: cpu.id as usize,
                    })
                })
                .collect(),
        )
        .unwrap();
}

pub fn init_local(local_cpu: &limine::mp::Cpu) {
    GsBase::write(VirtAddr::from_ptr(
        CPU_LOCAL_DATA
            .try_get()
            .unwrap()
            .iter()
            .find(|cpu| unsafe { cpu.get().read().cpu_id == local_cpu.id as usize })
            .unwrap(),
    ));
}

pub fn get_local() -> &'static SyncUnsafeCell<CpuLocalData> {
    let cpu_local_data_ptr = GsBase::read().as_ptr::<SyncUnsafeCell<CpuLocalData>>();
    let cpu_local_data = unsafe { &*cpu_local_data_ptr };
    cpu_local_data
}

/// Set the value that `rsp` will be set to when transitioning from user mode to kernel mode through the `syscall` instruction
pub fn set_syscall_stack_pointer(stack_pointer: VirtAddr) {
    let cpu_local_data = unsafe { &mut *get_local().get() };
    cpu_local_data.kernel_stack_pointer = stack_pointer.as_u64();
}
