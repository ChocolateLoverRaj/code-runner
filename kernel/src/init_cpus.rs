use limine::response::MpResponse;

use crate::hlt_loop::hlt_loop;

pub fn init_cpus(mp_response: &mut MpResponse) -> ! {
    mp_response.cpus_mut().iter_mut().for_each(|cpu| {
        cpu.goto_address.write(cpu_init);
    });

    let current_cpu = mp_response
        .cpus()
        .iter()
        .find(|cpu| cpu.lapic_id == mp_response.bsp_lapic_id())
        .unwrap();
    unsafe { cpu_init(current_cpu) }
}

unsafe extern "C" fn cpu_init(cpu: &limine::mp::Cpu) -> ! {
    log::info!("Hello from CPU: {:?}. LAPIC ID: {:?}", cpu.id, cpu.lapic_id);
    hlt_loop()
}
