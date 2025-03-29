use acpi::AcpiTables;
use thiserror::Error;
use x2apic::lapic::cpu_has_x2apic;
use x86_64::{
    structures::paging::{mapper::MapToError, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::{
    hhdm_offset::HhdmOffset,
    pt_allocator_2::{
        get_offset_page_table::get_offset_page_table, pt_frame_allocator_2::PtFrameAllocator2,
        KERNEL_ADDRESS_SPACE_TRACKER, PHYS_MEM_TRACKER,
    },
};

#[derive(Debug, Error)]
pub enum MapLocalXapicError {
    #[error("The interrupt model is not APIC")]
    InterruptModelNotApic,
    #[error("Failed to allocate virtual memory")]
    VirtualMemoryAllocationFailed,
    #[error("Error mapping memory")]
    MapError(MapToError<Size4KiB>),
}

/// If the CPU does not have x2apic, maps the local apic in memory
pub fn map_local_xapic<H: acpi::AcpiHandler>(
    acpi_tables: &AcpiTables<H>,
    hhdm_offset: HhdmOffset,
) -> Result<Option<LocalXapicVirtAddr>, MapLocalXapicError> {
    if cpu_has_x2apic() {
        Ok(None)
    } else {
        let xapic_base_addr = match acpi_tables.platform_info().unwrap().interrupt_model {
            acpi::InterruptModel::Apic(apic) => Ok(apic.local_apic_address),
            _ => Err(MapLocalXapicError::InterruptModelNotApic),
        }?;
        // The local apic base address is guaranteed to be aligned to a 4KiB page
        // And we need to map exactly 1 4KiB page
        let phys_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(xapic_base_addr));
        let mut virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let virt_start_number = virt_mem
            .get_continuous_range_with_alignment(false, 0x1000, 0x1000)
            .ok_or(MapLocalXapicError::VirtualMemoryAllocationFailed)?;
        virt_mem.set(virt_start_number..virt_start_number + 0x1000, true);
        let virt_start = VirtAddr::new_truncate(virt_start_number as u64);
        let page = Page::<Size4KiB>::from_start_address(virt_start).unwrap();
        let mut offset_page_table = get_offset_page_table(hhdm_offset);
        let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        let mut used_bytes = 0;
        let mut frame_allocator = PtFrameAllocator2 {
            phys_mem: &mut phys_mem,
            used_bytes: &mut used_bytes,
        };
        unsafe {
            offset_page_table.map_to(
                page,
                phys_frame,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::NO_CACHE
                    | PageTableFlags::NO_EXECUTE,
                &mut frame_allocator,
            )
        }
        .map_err(|e| MapLocalXapicError::MapError(e))?
        .flush();
        Ok(Some(LocalXapicVirtAddr(virt_start)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalXapicVirtAddr(VirtAddr);

impl From<LocalXapicVirtAddr> for VirtAddr {
    fn from(virt_addr: LocalXapicVirtAddr) -> Self {
        virt_addr.0
    }
}
