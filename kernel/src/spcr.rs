use acpi::{
    address::AddressSpace,
    spcr::{Spcr, SpcrInteraceType},
};
use x86_64::PhysAddr;

/// Get the physical base address that you can use with the `uart_16550` crate. If the console is not 16550 compatible or isn't accessed through physical memory, returns `None`.
pub fn get_16550_base_address(spcr: &Spcr) -> Option<PhysAddr> {
    match spcr.interface_type() {
        SpcrInteraceType::Full16550
        | SpcrInteraceType::Full16450
        | SpcrInteraceType::Generic16550 => Some(()),
        _ => None,
    }?;
    let base_address = spcr.base_address()?.ok()?;
    match base_address.address_space {
        AddressSpace::SystemMemory => Some(()),
        _ => None,
    }?;
    match base_address.bit_offset {
        0 => Some(()),
        _ => None,
    }?;
    Some(PhysAddr::new(base_address.address))
}
