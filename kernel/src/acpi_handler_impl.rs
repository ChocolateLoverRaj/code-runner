use core::{cell::RefCell, ops::DerefMut, ptr::NonNull};

use acpi::AcpiHandler;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::{
    available_physical_frame_iterator::AvailablePhysicalFrameIteratorFrameAllocator,
    find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table, hhdm_offset::HhdmOffset,
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
};

#[derive(Debug, Clone)]
pub struct AcpiHandlerImpl<'a> {
    hhdm_offset: HhdmOffset,
    frame_allocator: &'a RefCell<AvailablePhysicalFrameIteratorFrameAllocator>,
}

impl<'a> AcpiHandlerImpl<'a> {
    pub const fn new(
        hhdm_offset: HhdmOffset,
        frame_allocator: &'a RefCell<AvailablePhysicalFrameIteratorFrameAllocator>,
    ) -> Self {
        Self {
            hhdm_offset,
            frame_allocator,
        }
    }
}

impl AcpiHandler for AcpiHandlerImpl<'_> {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        log::debug!(
            "Mapping phys: 0x{:X} with len 0x{:X}",
            physical_address,
            size
        );
        let page_count =
            (physical_address + size).div_ceil(0x1000) - physical_address.div_floor(0x1000);
        let pages = find_contiguous_unused_virtual_memory(
            unsafe { PageTablesRecursiveIterator::new(self.hhdm_offset, Cr3::read().0, 256) },
            page_count as u64,
        )
        .unwrap();
        // FIXME: we can't assume it's page aligned
        let first_phys_frame =
            PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(physical_address as u64));
        let mut offset_page_table = get_offset_page_table(self.hhdm_offset.into());

        for i in 0..page_count {
            unsafe {
                offset_page_table.map_to(
                    pages.start + i as u64,
                    first_phys_frame + i as u64,
                    PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE | PageTableFlags::NO_CACHE,
                    self.frame_allocator.borrow_mut().deref_mut(),
                )
            }
            .unwrap()
            .flush();
        }
        let mapped_length_from_start_ptr = page_count * 0x1000;
        unsafe {
            acpi::PhysicalMapping::new(
                physical_address,
                // FIXME: we can't assume it's page aligned
                NonNull::new(pages.start.start_address().as_mut_ptr()).unwrap(),
                size,
                // TODO: Actual mapped len may be more than this, could improve performance to give actual mapped len
                mapped_length_from_start_ptr,
                self.clone(),
            )
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        let start_addr = VirtAddr::from_ptr(region.virtual_start().as_ptr());
        let end_addr_exclusive = start_addr + region.mapped_length() as u64;
        let start_page = Page::<Size4KiB>::containing_address(start_addr);
        let end_page = Page::<Size4KiB>::containing_address(end_addr_exclusive - 1);
        let mut offset_page_table = get_offset_page_table(region.handler().hhdm_offset.into());
        for page in start_page..=end_page {
            offset_page_table.unmap(page).unwrap().1.flush();
        }
    }
}
