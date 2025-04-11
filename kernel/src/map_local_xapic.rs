use core::{cell::RefCell, ops::DerefMut};

use acpi::{AcpiHandler, AcpiTables};
use thiserror::Error;
use x2apic::lapic::cpu_has_x2apic;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::{
    find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table, hhdm_offset::HhdmOffset,
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
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
pub fn map_local_xapic(
    acpi_tables: &AcpiTables<impl AcpiHandler>,
    hhdm_offset: HhdmOffset,
    frame_allocator: &RefCell<impl FrameAllocator<Size4KiB>>,
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
        let phys_frame =
            PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(xapic_base_addr)).unwrap();
        let page = find_contiguous_unused_virtual_memory(
            unsafe { PageTablesRecursiveIterator::new(hhdm_offset, Cr3::read().0, 256) },
            1,
        )
        .unwrap()
        .start;
        let mut offset_page_table = get_offset_page_table(hhdm_offset);
        unsafe {
            offset_page_table.map_to(
                page,
                phys_frame,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::NO_CACHE
                    | PageTableFlags::NO_EXECUTE,
                frame_allocator.borrow_mut().deref_mut(),
            )
        }
        .unwrap()
        .flush();
        Ok(Some(LocalXapicVirtAddr(page.start_address())))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LocalXapicVirtAddr(VirtAddr);

impl From<LocalXapicVirtAddr> for VirtAddr {
    fn from(virt_addr: LocalXapicVirtAddr) -> Self {
        virt_addr.0
    }
}
