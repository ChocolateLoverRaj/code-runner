use core::{cell::SyncUnsafeCell, ptr::NonNull};

use alloc::boxed::Box;
use limine::{mp::Cpu, response::MpResponse};
use util::init_later::InitLater;
use x86_64::{registers::model_specific::GsBase, VirtAddr};

/// This is what we set `GS.Base` to point to
#[derive(Debug)]
pub struct CpuLocalData {
    // pub user_stack_pointer: u64,
    // pub kernel_stack_pointer: u64,
    pub cpu_id: u32,
}

impl From<&Cpu> for CpuLocalData {
    fn from(value: &Cpu) -> Self {
        Self { cpu_id: value.id }
    }
}

static CPU_LOCAL_DATA: InitLater<Box<[SyncUnsafeCell<CpuLocalData>]>> = InitLater::uninit();

pub fn init_bsp(mp_response: &MpResponse) {
    CPU_LOCAL_DATA
        .try_init(
            mp_response
                .cpus()
                .iter()
                .map(|&cpu| SyncUnsafeCell::new(cpu.into()))
                .collect(),
        )
        .unwrap();
}

/// # Safety
/// The GS base register is set to the address of the CPU local data.
pub unsafe fn init_cpu(local_cpu: &limine::mp::Cpu) {
    GsBase::write(VirtAddr::from_ptr(
        CPU_LOCAL_DATA
            .try_get()
            .unwrap()
            .iter()
            .find(|cpu| {
                // Safety: Nothing is modifying the CPU data while we are reading it.
                unsafe { cpu.get().read() }.cpu_id == local_cpu.id
            })
            .unwrap(),
    ));
}

pub fn get_local() -> Option<&'static SyncUnsafeCell<CpuLocalData>> {
    if CPU_LOCAL_DATA.is_initialized() {
        Some({
            let cpu_local_data_ptr =
                NonNull::new(GsBase::read().as_mut_ptr::<SyncUnsafeCell<CpuLocalData>>()).unwrap();
            // Safety: The GS base register is set to the address of the CPU local data.
            let cpu_local_data = unsafe { cpu_local_data_ptr.as_ref() };
            cpu_local_data
        })
    } else {
        None
    }
}

// /// Set the value that `rsp` will be set to when transitioning from user mode to kernel mode through the `syscall` instruction
// pub fn set_syscall_stack_pointer(stack_pointer: VirtAddr) {
//     let cpu_local_data = unsafe { &mut *get_local().get() };
//     cpu_local_data.kernel_stack_pointer = stack_pointer.as_u64();
// }
