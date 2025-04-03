use core::ptr::NonNull;

use acpi::AcpiHandler;
use x86_64::{
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::{
    hhdm_offset::HhdmOffset,
    pt_allocator_2::{
        get_offset_page_table::get_offset_page_table, pt_frame_allocator_3::PtFrameAllocator3,
        KERNEL_ADDRESS_SPACE_TRACKER, MEMORY_USAGE_STATS,
    },
};

#[derive(Debug, Clone)]
pub struct AcpiHandlerImpl {
    pub hhdm_offset: HhdmOffset,
}

impl AcpiHandler for AcpiHandlerImpl {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        log::info!(
            "Mapping phys: 0x{:X} with len 0x{:X}",
            physical_address,
            size
        );
        let mut virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let mut mem_usage = MEMORY_USAGE_STATS.try_get().unwrap().lock();

        let page_count =
            (physical_address + size).div_ceil(0x1000) - physical_address.div_floor(0x1000);
        let virt_start = virt_mem
            .get_continuous_range_with_alignment(false, page_count * 0x1000, 0x1000)
            .unwrap();
        virt_mem.set(virt_start..virt_start + page_count * 0x1000, true);
        let first_phys_frame =
            PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(physical_address as u64));
        let virt_start = VirtAddr::new_truncate(virt_start as u64);
        let first_page = Page::<Size4KiB>::from_start_address(virt_start).unwrap();
        let mut offset_page_table = get_offset_page_table(self.hhdm_offset.into());
        let mut frame_allocator = PtFrameAllocator3 {
            f: |_| {
                mem_usage.page_tables += 0x1000;
            },
        };

        for i in 0..page_count {
            unsafe {
                offset_page_table.map_to(
                    first_page + i as u64,
                    first_phys_frame + i as u64,
                    PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE | PageTableFlags::NO_CACHE,
                    &mut frame_allocator,
                )
            }
            .unwrap()
            .flush();
        }
        let mapped_length_from_start_ptr =
            ((first_page + page_count as u64).start_address() - virt_start) as usize;
        unsafe {
            acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new(virt_start.as_mut_ptr()).unwrap(),
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
            offset_page_table.unmap(page).unwrap().1.ignore();
        }
    }
}
