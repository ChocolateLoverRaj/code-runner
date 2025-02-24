use core::{
    ops::{DerefMut, Range},
    ptr::NonNull,
};

use acpi::AcpiHandler;
use alloc::sync::Arc;
use x86_64::{
    structures::paging::{
        frame::PhysFrameRange, page::PageRange, Mapper, OffsetPageTable, Page, PageSize,
        PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::{memory::BootInfoFrameAllocator, virt_mem_tracker::VirtMemTracker};

/// Used to "create" a virtual address range that maps to a specified physical address range
#[derive(Clone, Debug)]
pub struct PhysMapper {
    phys_mem_offset: u64,
    mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
    virt_mem_tracker: Arc<spin::Mutex<VirtMemTracker>>,
    frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
}
impl PhysMapper {
    pub fn new(
        phys_mem_offset: u64,
        mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
        virt_mem_tracker: Arc<spin::Mutex<VirtMemTracker>>,
        frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
    ) -> Self {
        Self {
            phys_mem_offset,
            mapper,
            virt_mem_tracker,
            frame_allocator,
        }
    }

    /// # Safety
    /// Don't cause memory leaks by forgetting to unmap it
    pub unsafe fn map_to_phys(
        &self,
        phys_frame_range: PhysFrameRange,
        flags: PageTableFlags,
    ) -> PageRange {
        let page_count = phys_frame_range.end - phys_frame_range.start;
        // log::info!(
        //     "Page count: {page_count}. Tracker: {:#?}",
        //     self.virt_mem_tracker.lock()
        // );
        let start_page = self
            .virt_mem_tracker
            .lock()
            .allocate_pages::<Size4KiB>(page_count)
            .expect("Failed to allocate virt addr");
        let page_range = Page::range(start_page, start_page + page_count);
        let mut mapper = self.mapper.lock();
        let mut flush = None;
        for page_index in 0..page_count {
            flush = Some(
                unsafe {
                    mapper.map_to(
                        page_range.clone().nth(page_index as usize).unwrap(),
                        phys_frame_range.clone().nth(page_index as usize).unwrap(),
                        flags,
                        self.frame_allocator.lock().deref_mut(),
                    )
                }
                .expect("Error mapping frame"),
            );
        }
        flush.unwrap().flush();
        page_range
    }

    /// # Safety
    /// Only unmap pages that were mapped by this and are not already unmapped
    pub unsafe fn unmap(&self, page_range: Range<Page>) {
        let mut mapper = self.mapper.lock();
        let mut flush = None;
        for page in page_range.clone() {
            flush = Some(mapper.unmap(page).unwrap().1);
        }
        flush.unwrap().flush();
        let mut ranges = self.virt_mem_tracker.lock();
        ranges.deallocate_pages_unchecked(page_range);
    }
}

impl AcpiHandler for PhysMapper {
    // FIXME: This is buggy and causes a page fault on Jinlon (probably)
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        // let frame_range = {
        //     let start_frame = PhysFrame::containing_address(PhysAddr::new(physical_address as u64));
        //     PhysFrame::range(start_frame, start_frame + 1)
        // };
        // let page_range = unsafe {
        //     self.map_to_phys(
        //         frame_range,
        //         PageTableFlags::PRESENT
        //             | PageTableFlags::WRITABLE
        //             | PageTableFlags::NO_CACHE
        //             | PageTableFlags::WRITE_THROUGH,
        //     )
        // };
        // log::debug!("AcpiHandler map: {page_range:?} pointing to {frame_range:?}");
        unsafe {
            acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new((physical_address + self.phys_mem_offset as usize) as *mut T).unwrap(), //page must exist
                size,
                size, // Could be bigger, but leaving it as size to not mess things up
                self.clone(),
            )
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        // let frame_range = {
        //     let start_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
        //         region.physical_start() as u64,
        //     ));
        //     let end_frame = PhysFrame::containing_address(PhysAddr::new(
        //         region.physical_start() as u64 + region.region_length() as u64,
        //     ));
        //     PhysFrame::range(start_frame, end_frame + 1)
        // };
        // let page_count = frame_range.len();
        // let mapper = region.handler();
        // // Address may not be aligned
        // let start_page = Page::<Size4KiB>::containing_address(VirtAddr::new(
        //     region.virtual_start().as_ptr() as u64,
        // ));
        // let page_range = start_page..start_page + page_count;
        // log::debug!("AcpiHandler unmap: {page_range:?}");
        // unsafe { mapper.unmap(page_range) };
    }
}
