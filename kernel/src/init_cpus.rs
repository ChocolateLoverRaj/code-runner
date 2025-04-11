use common::ram_disk::RamDisk;
use util::init_later::InitLater;

use crate::{cpu_local_data, hlt_loop::hlt_loop, init_idt_and_gdt, limine_requests::MP_REQUEST};

static RAM_DISK: InitLater<RamDisk<'static>> = InitLater::uninit();

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
    x86_64::instructions::interrupts::int3();
    todo!("Stuff");
    hlt_loop()
}
