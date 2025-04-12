use core::ptr::NonNull;

use alloc::boxed::Box;
use limine::response::MpResponse;
use spinning_top::Spinlock;
use util::init_later::InitLater;
use x2apic::lapic::LocalApic;
use x86_64::{registers::model_specific::GsBase, VirtAddr};

use crate::{
    init_idt_and_gdt::{StaticStuff1, StaticStuff2},
    store_but_borrow_mut::StoreButBorrowMut,
    tasks::CpuTaskData,
};

/// This is what we set `GS.Base` to point to
#[derive(Debug)]
pub struct CpuLocalData {
    // pub user_stack_pointer: u64,
    // pub kernel_stack_pointer: u64,
    pub cpu_id: u32,
    pub static_stuff1: StoreButBorrowMut<StaticStuff1>,
    pub static_stuff2: InitLater<StaticStuff2>,
    pub local_apic: InitLater<Spinlock<LocalApic>>,
    pub task_data: Spinlock<CpuTaskData>,
}

static CPU_LOCAL_DATA: InitLater<Box<[CpuLocalData]>> = InitLater::uninit();

pub fn init_bsp(mp_response: &MpResponse) {
    CPU_LOCAL_DATA
        .try_init(
            mp_response
                .cpus()
                .iter()
                .map(|&cpu| CpuLocalData {
                    cpu_id: cpu.id,
                    static_stuff1: StoreButBorrowMut::uninit(),
                    static_stuff2: InitLater::uninit(),
                    local_apic: InitLater::uninit(),
                    task_data: Default::default(),
                })
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
            .find(|cpu| cpu.cpu_id == local_cpu.id)
            .unwrap(),
    ));
}

pub fn get_local() -> Option<&'static CpuLocalData> {
    if CPU_LOCAL_DATA.is_initialized() {
        Some({
            let cpu_local_data_ptr =
                NonNull::new(GsBase::read().as_mut_ptr::<CpuLocalData>()).unwrap();
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
