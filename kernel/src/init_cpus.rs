use limine::response::{ModuleResponse, MpResponse};

use crate::{
    cpu_local_data,
    get_total_memory::get_memory_usage_stats,
    hhdm_offset::HhdmOffset,
    hlt_loop::hlt_loop,
    init_idt_and_gdt::{init_idt_and_gdt, init_vars_for_idt_and_gdt},
    parse_ram_disk::parse_ram_disk,
    rsdp_addr::RsdpAddr,
};

pub fn init_cpus(
    mp_response: &mut MpResponse,
    rsdp_addr: RsdpAddr,
    hhdm_offset: HhdmOffset,
    module_response: Option<&ModuleResponse>,
) -> ! {
    cpu_local_data::init(mp_response);
    init_vars_for_idt_and_gdt(rsdp_addr, hhdm_offset);

    let ram_disk = parse_ram_disk(module_response.unwrap()).unwrap();
    log::info!("Ram disk meta: {:#?}", ram_disk.meta_data);

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
    // This is probably needed cuz the allocator changed page tables. It did not change the location of the L4 page table though.
    x86_64::instructions::tlb::flush_all();

    cpu_local_data::init_local(cpu);

    log::info!("Hello from CPU: {:?}. LAPIC ID: {:?}", cpu.id, cpu.lapic_id);

    init_idt_and_gdt();

    log::info!("Initialized Local APIC on CPU {}", cpu.id);
    log::info!("{:#?}", get_memory_usage_stats());

    x86_64::instructions::interrupts::int3();
    // if cpu.id == 0 {
    //     for i in 0..500_000_000 {}
    //     panic!("Test panic");
    // }

    // loop {
    //     log::info!("Log from CPU {:?}", cpu.id);
    // }

    hlt_loop()
}
