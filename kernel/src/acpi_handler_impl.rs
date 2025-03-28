use acpi::AcpiHandler;
use x86_64::{
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::pt_allocator_2::{
    get_offset_page_table::get_offset_page_table, KERNEL_ADDRESS_SPACE_TRACKER,
};

#[derive(Debug, Clone)]
pub struct AcpiHandlerImpl {
    hhdm_offset: u64,
}

impl AcpiHandler for AcpiHandlerImpl {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();

        let page_count =
            (physical_address + size - 1).div_ceil(0x1000) - physical_address.div_floor(0x1000);
        let virt_start = virt_mem
            .get_continuous_range_with_alignment(false, page_count * 0x1000, 0x1000)
            .unwrap();
        virt_mem.set(virt_start..virt_start + page_count * 0x1000, true);
        let first_phys_frame =
            PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(physical_address as u64));
        let first_page =
            Page::<Size4KiB>::from_start_address(VirtAddr::new_truncate(virt_start as u64))
                .unwrap();
        let mut offset_page_table = get_offset_page_table(self.hhdm_offset);
        for i in 0..page_count {
            offset_page_table.map_to(
                first_page + i,
                first_phys_frame + i,
                PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE | PageTableFlags::NO_CACHE,
                frame_allocator,
            );
        }
    }
}
