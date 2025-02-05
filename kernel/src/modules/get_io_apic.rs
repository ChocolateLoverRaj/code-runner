use acpi::platform::interrupt::Apic;
use alloc::alloc::Global;
use x2apic::ioapic::IoApic;
use x86_64::{
    structures::paging::{PageTableFlags, PhysFrame},
    PhysAddr,
};

use crate::phys_mapper::PhysMapper;

/// # Safety
/// Calls `IoApic::new`
pub unsafe fn get_io_apic(apic: &Apic<Global>, phys_mapper: &mut PhysMapper) -> IoApic {
    // Map IO APIC
    // From https://wiki.osdev.org/APIC#IO_APIC_Registers, there are 64 32-bit registers, so 256 bytes need to be mapped to access the IO APIC. We can map a single frame.
    let phys_frame_range = {
        let frame = PhysFrame::containing_address(PhysAddr::new(apic.io_apics[0].address as u64));
        PhysFrame::range(frame, frame + 1)
    };
    let flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_CACHE
        | PageTableFlags::NO_EXECUTE;
    let io_mapping = unsafe { phys_mapper.map_to_phys(phys_frame_range, flags) };
    let base_addr = io_mapping.start.start_address().as_u64();
    unsafe { IoApic::new(base_addr) }
}
