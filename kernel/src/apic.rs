// use acpi::AcpiTables;
use bootloader_api::info::Optional;

pub fn init_apic(rsdp_addr: Optional<u64>) {
    log::info!("RSDP: {:?}", rsdp_addr.as_ref());
    // AcpiTables::from_rsdp(handler, address)
}
