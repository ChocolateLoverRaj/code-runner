use core::ptr::NonNull;

use acpi::{AcpiHandler, AcpiTables, HpetInfo};
use anyhow::anyhow;
use volatile::VolatilePtr;
use x86_64::{
    instructions::interrupts,
    structures::paging::{frame::PhysFrameRange, PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr,
};

use crate::{
    hlt_loop::hlt_loop,
    hpet_memory::{HpetMemory, HpetMemoryVolatileFieldAccess, HpetTimerMemoryVolatileFieldAccess},
    phys_mapper::PhysMapper,
};

pub fn init<H: AcpiHandler>(
    acpi_tables: &AcpiTables<H>,
    phys_mapper: PhysMapper,
) -> anyhow::Result<()> {
    let hpet_info = HpetInfo::new(acpi_tables).map_err(|e| anyhow!("{:?}", e))?;
    let virt_mapping = unsafe {
        phys_mapper.map_to_phys(
            {
                let start = PhysAddr::new(hpet_info.base_address as u64);
                PhysFrameRange {
                    start: PhysFrame::containing_address(start),
                    end: PhysFrame::containing_address(start + size_of::<HpetMemory>() as u64) + 1,
                }
            },
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE
                | PageTableFlags::NO_EXECUTE,
        )
    };
    let virt_start =
        virt_mapping.start.start_address() + (hpet_info.base_address as u64 % Size4KiB::SIZE);

    let hpet_volatile_ptr =
        unsafe { VolatilePtr::<HpetMemory>::new(NonNull::new(virt_start.as_mut_ptr()).unwrap()) };
    log::info!("HPET info: {:#?}", hpet_info);
    let period_femtoseconds = hpet_volatile_ptr
        .capabilities_and_id()
        .read()
        .get_counter_clk_period();
    let number_of_timers = hpet_volatile_ptr
        .capabilities_and_id()
        .read()
        .get_num_tim_cap()
        + 1;
    log::info!("HPET has {} timers", number_of_timers);

    // Enable the first timer
    let timer = hpet_volatile_ptr.timers().as_slice().index(0);
    timer
        .configuration_and_capability_register()
        .update(|mut r| {
            r.set_int_route_cnf(2);
            r.set_int_enb_cnf(true);
            r
        });
    timer.comparator_register().write(293733257);

    hpet_volatile_ptr.config().update(|mut config| {
        config.set_enable_cnf(true);
        config
    });

    let comparator = timer.comparator_register().read();
    let timer = timer.configuration_and_capability_register().read();
    log::info!("TImer 0: {:#?}. Comparator: {}", timer, comparator);

    interrupts::enable();
    log::info!(
        "HPET general config: {:#?}. INterrupts enabled?: {}",
        hpet_volatile_ptr.config().read(),
        interrupts::are_enabled()
    );

    hlt_loop();
    loop {
        let counter = hpet_volatile_ptr.main_counter_value_register().read();
        let counter_in_femtoseconds = counter * period_femtoseconds as u64;
        log::info!(
            "Main counter value: {}. Period (in s^-15): {}. Counter in (s^-15): {}",
            counter,
            period_femtoseconds,
            counter_in_femtoseconds
        );
    }
    Ok(())
}
