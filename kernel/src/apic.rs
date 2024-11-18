use core::ptr::NonNull;

use acpi::{AcpiHandler, AcpiTables, InterruptModel};
use x2apic::{
    ioapic::{IoApic, IrqFlags, IrqMode, RedirectionTableEntry},
    lapic::{LocalApic, LocalApicBuilder},
};
use x86_64::{instructions::interrupts::without_interrupts, VirtAddr};

#[derive(Clone, Debug)]
struct OffsetMapAcpiHandler {
    phys_map_offset: VirtAddr,
}

impl AcpiHandler for OffsetMapAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        acpi::PhysicalMapping::new(
            physical_address,
            NonNull::new((self.phys_map_offset + physical_address as u64).as_mut_ptr::<T>())
                .unwrap(), //page must exist
            size,
            size,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {}
}

pub static LOCAL_APIC: spin::Mutex<Option<LocalApic>> = spin::Mutex::new(None);

pub fn init_apic(rsdp_addr: usize, phys_map_offset: VirtAddr) {
    log::info!("RSDP: {:?}", rsdp_addr);
    let acpi_tables =
        unsafe { AcpiTables::from_rsdp(OffsetMapAcpiHandler { phys_map_offset }, rsdp_addr) }
            .unwrap();
    let platform_info = acpi_tables.platform_info().unwrap();
    let interrupt_model = platform_info.interrupt_model;
    // log::info!("ACPI Tables: {:?}", interrupt_model);
    match interrupt_model {
        InterruptModel::Apic(apic) => {
            without_interrupts(|| {
                log::info!("APIC: {:?}", apic);
                let mut local_apic = LocalApicBuilder::new()
                    .timer_vector(41)
                    .error_vector(42)
                    .spurious_vector(43)
                    .set_xapic_base((phys_map_offset + apic.local_apic_address).as_u64())
                    .build()
                    .unwrap();
                // unsafe {
                //     local_apic.enable();
                // };

                match apic.io_apics.first() {
                    Some(io_apic) => {
                        let mut io_apic = unsafe {
                            x2apic::ioapic::IoApic::new(
                                (phys_map_offset + io_apic.address as u64).as_u64(),
                            )
                        };
                        unsafe { io_apic.init(42) };
                        for i in 0..(255 - 42) {
                            let mut entry = RedirectionTableEntry::default();
                            entry.set_mode(IrqMode::Fixed);
                            entry.set_flags(IrqFlags::LEVEL_TRIGGERED | IrqFlags::LOW_ACTIVE);
                            entry.set_vector(i);
                            entry.set_dest(unsafe { local_apic.id() } as u8);
                            unsafe {
                                io_apic.set_table_entry(i, entry);
                                io_apic.enable_irq(i);
                            };
                        }
                    }
                    None => {
                        log::warn!("No IO APIC");
                    }
                }

                *LOCAL_APIC.lock() = Some(local_apic);
            });
        }
        _ => {
            log::warn!("Unknown interrupt model.");
        }
    }
}
