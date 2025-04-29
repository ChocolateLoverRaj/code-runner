use alloc::boxed::Box;
use spinning_top::Spinlock;

use crate::{
    cpu_local_data::{self, get_local},
    init_idt_and_gdt,
    limine_requests::{FRAME_BUFFER_REQUEST, HHDM_REQUEST, MP_REQUEST},
    modules::syscall::init_syscalls::init_syscalls,
    run_tasks::run_tasks,
    syscalls::{
        raw_syscall_handler::set_syscall_handler_closure,
        syscall_handler_closure::get_syscall_handlers,
    },
    tasks::CPU_TASK_STATES,
};

/// # Safety
/// Must be called exactly once, after BSP init
pub unsafe fn init_cpus() -> ! {
    let mp_response = unsafe {
        #[allow(static_mut_refs)]
        MP_REQUEST.get_response_mut().unwrap()
    };
    cpu_local_data::init_bsp(mp_response);
    mp_response.cpus_mut().iter_mut().for_each(|cpu| {
        cpu.goto_address.write(init_cpu);
    });
    let current_cpu = mp_response
        .cpus()
        .iter()
        .find(|cpu| cpu.lapic_id == mp_response.bsp_lapic_id())
        .unwrap();
    CPU_TASK_STATES
        .try_init(Spinlock::new(
            mp_response.cpus().iter().map(|_| None).collect(),
        ))
        .unwrap();
    unsafe { init_cpu(current_cpu) }
}

unsafe extern "C" fn init_cpu(cpu: &limine::mp::Cpu) -> ! {
    // Safety: We are only using GS.Base for CPU local data
    unsafe { cpu_local_data::init_cpu(cpu) };
    log::info!(
        "Hello from CPU 0x{:02X}. Local APIC ID: 0x{:02X}",
        cpu.id,
        cpu.lapic_id
    );
    init_idt_and_gdt::init_cpu();
    log::info!("Set up idt and gdt!");
    // x86_64::instructions::interrupts::int3();
    let hhdm_offset = (&HHDM_REQUEST).try_into().unwrap();
    get_local()
        .unwrap()
        .initialized_syscalls
        .try_init(init_syscalls(set_syscall_handler_closure(Box::new(
            get_syscall_handlers(hhdm_offset, FRAME_BUFFER_REQUEST.get_response()),
        ))))
        .unwrap();
    if cpu.id == 0 {
        // for i in 0..100000000 {}
    }
    run_tasks()
}
