use common::ram_disk::RamDisk;
use limine::response::{ModuleResponse, MpResponse};
use util::init_later::InitLater;

use crate::{
    cpu_local_data,
    get_total_memory::get_memory_usage_stats,
    hhdm_offset::HhdmOffset,
    init_idt_and_gdt::{init_idt_and_gdt, init_vars_for_idt_and_gdt},
    limine_requests::HHDM_REQUEST,
    parse_ram_disk::parse_ram_disk,
    rsdp_addr::RsdpAddr,
    run_tasks::run_tasks,
    spawn_task::spawn_task,
    tasks::{try_init_cpu_task_data, try_init_tasks, TASKS},
};

static RAM_DISK: InitLater<RamDisk<'static>> = InitLater::uninit();

pub fn init_cpus(
    mp_response: &mut MpResponse,
    rsdp_addr: RsdpAddr,
    hhdm_offset: HhdmOffset,
    module_response: Option<&'static ModuleResponse>,
) -> ! {
    cpu_local_data::init(mp_response);
    init_vars_for_idt_and_gdt(rsdp_addr, hhdm_offset);

    let ram_disk = parse_ram_disk(module_response.unwrap()).unwrap();
    RAM_DISK.try_init(ram_disk).unwrap();
    try_init_tasks().unwrap();
    try_init_cpu_task_data().unwrap();

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
    log::info!("Spawning task");
    // if cpu.id == 0 {
    //     for i in 0..500_000_000 {}
    //     panic!("Test panic");
    // }

    // loop {
    //     log::info!("Log from CPU {:?}", cpu.id);
    // }

    let hhdm_offset = (&HHDM_REQUEST).try_into().unwrap();
    if cpu.id == 0 {
        spawn_task(RAM_DISK.try_get().unwrap(), hhdm_offset);
        {
            let tasks = TASKS.try_get().unwrap().lock();
            log::info!(
                "Spawned task. {:#?}",
                tasks.tasks.first().unwrap().owned_phys_mem
            );
        }
    }
    run_tasks()

    // hlt_loop()
}
