use core::{alloc::GlobalAlloc, ops::DerefMut};

use limine::{memory_map::EntryType, response::MemoryMapResponse};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};

use crate::{
    pt_allocator_2::{
        get_offset_page_table::get_offset_page_table, pt_frame_allocator::PtFrameAllocator,
        PHYS_MEM_TRACKER, PHYS_MEM_USED_BY_KERNEL,
    },
    virt_addr_to_number::VirtAddrToNumber,
};

use super::KERNEL_ADDRESS_SPACE_TRACKER;

pub struct PtAllocator2 {
    pub(crate) memory_map_response: &'static MemoryMapResponse,
    pub(crate) hhdm_offset: u64,
}

unsafe impl GlobalAlloc for PtAllocator2 {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // First we find the first phys frame
        let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        let mut virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let mut used_phys_bytes = 0;
        let first_frame_portion = phys_mem
            .iter()
            .filter(|segment| !segment.value)
            .find_map(|segment| {
                let start = segment.position.next_multiple_of(layout.align());
                let end = (start + 1)
                    .next_multiple_of(0x1000)
                    .min(start + layout.size());
                log::info!(
                    "Pos: {}, Start: {}. Align: {}. End: {}",
                    segment.position,
                    start,
                    layout.align(),
                    end,
                );
                if end <= segment.position + segment.len {
                    Some(start..end)
                } else {
                    None
                }
            })
            .unwrap();
        log::info!(
            "Using phys range: {:X?}. Layout: {:#?}. Phys Tracker: {:#?}",
            first_frame_portion,
            layout,
            phys_mem
        );
        phys_mem.set(first_frame_portion.clone(), true);
        // log::info!("Phys mem: {:#?}", phys_mem);
        used_phys_bytes += first_frame_portion.len();

        // If the entire layout is in a single phys frame, we can just use a pointer to an offset-mapped page
        if first_frame_portion.len() == layout.size() {
            // log::info!("Allocated data is in a single phys frame. Returning a pointer to an offset-mapped page instead of reserving virt mem.");
            VirtAddr::new_truncate(first_frame_portion.start as u64 + self.hhdm_offset).as_mut_ptr()
        } else {
            // Find continuous virt range (virt must be continuous, phys only has to be made of continuous 4KiB chunks)
            let virt_range = virt_mem
                .iter()
                .filter(|segment| !segment.value)
                .find_map(|segment| {
                    let phys_offset_in_4kib = first_frame_portion.start % 0x1000;
                    let virt_start = segment.position.next_multiple_of(0x1000);
                    let virt_end = virt_start + phys_offset_in_4kib + layout.size();
                    if virt_end <= segment.position + segment.len {
                        Some(virt_start + phys_offset_in_4kib..virt_end)
                    } else {
                        None
                    }
                })
                .unwrap();
            log::info!("Using virt range: {:X?}", virt_range);
            virt_mem.set(virt_range.clone(), true);

            // Map the first range
            let mut offset_page_table = get_offset_page_table(self.hhdm_offset);
            let mut frame_allocator = PtFrameAllocator {
                phys: phys_mem.deref_mut(),
            };
            let virt_start = VirtAddr::new_truncate(virt_range.start as u64);
            let first_page = Page::<Size4KiB>::containing_address(virt_start);
            unsafe {
                offset_page_table.map_to(
                    first_page,
                    PhysFrame::containing_address(PhysAddr::new(first_frame_portion.start as u64)),
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                    &mut frame_allocator,
                )
            }
            .unwrap()
            .flush();
            *PHYS_MEM_USED_BY_KERNEL.try_get().unwrap().lock() += used_phys_bytes;

            // Map rest of memory
            let mut remaining_to_map = layout.size() - first_frame_portion.len();
            let mut page_offset = 1;
            loop {
                if remaining_to_map == 0 {
                    break;
                }
                let map_len = remaining_to_map.min(0x1000);
                let phys_frame_start = phys_mem
                    .get_continuous_range_with_alignment(false, map_len, 0x1000)
                    .unwrap();
                phys_mem.set(phys_frame_start..phys_frame_start + map_len, true);
                let mut frame_allocator = PtFrameAllocator {
                    phys: phys_mem.deref_mut(),
                };
                unsafe {
                    offset_page_table.map_to(
                        first_page + page_offset,
                        PhysFrame::containing_address(PhysAddr::new(phys_frame_start as u64)),
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut frame_allocator,
                    )
                }
                .unwrap()
                .flush();
                remaining_to_map -= map_len;
                page_offset += 1;
            }

            virt_start.as_mut_ptr()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let is_hddm = {
            if let Some(phys_addr_if_hddm) = (ptr as u64).checked_sub(self.hhdm_offset) {
                self.memory_map_response
                    .entries()
                    .iter()
                    .filter(|entry| {
                        [
                            EntryType::USABLE,
                            EntryType::BOOTLOADER_RECLAIMABLE,
                            EntryType::EXECUTABLE_AND_MODULES,
                            EntryType::FRAMEBUFFER,
                        ]
                        .contains(&entry.entry_type)
                    })
                    .any(|entry| {
                        let start = entry.base;
                        let end = entry.base + entry.length;
                        (start..end).contains(&phys_addr_if_hddm)
                    })
            } else {
                false
            }
        };

        let mut phys_tracker = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        if is_hddm {
            // Just mark phys as unused, that's all
            let range = {
                let start = (VirtAddr::from_ptr(ptr) - self.hhdm_offset).into_number();
                start..start + layout.size()
            };
            log::info!("Deallocating HDDM - Marking phys: {:X?} as usable", range);
            phys_tracker.set(range, false);
        } else {
            let mut virt_tracker = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
            let mut bytes_deallocated = 0;
            let mut offset_page_table = get_offset_page_table(self.hhdm_offset);
            loop {
                if bytes_deallocated == layout.size() {
                    break;
                }
                let virt = VirtAddr::from_ptr(ptr) + bytes_deallocated as u64;
                let phys = offset_page_table.translate_addr(virt).unwrap();
                let bytes_left_to_deallocate = layout.size() - bytes_deallocated;
                let virt_offset_in_page = virt.as_u64() as usize % 0x1000;
                let bytes_deallocated_in_current_frame =
                    (0x1000 - virt_offset_in_page).min(bytes_left_to_deallocate);
                offset_page_table
                    .unmap(Page::<Size4KiB>::containing_address(virt))
                    .unwrap()
                    .1
                    .flush();
                {
                    let start = phys.as_u64() as usize;
                    let range = start..start + bytes_deallocated_in_current_frame;
                    log::info!("Marking phys as usable: {:X?}", range);
                    phys_tracker.set(range, false);
                }
                {
                    let start = virt.into_number();
                    let range = start..start + bytes_deallocated_in_current_frame;
                    log::info!("Marking virt as usable: {:X?}", range);
                    virt_tracker.set(range, false);
                }
                bytes_deallocated += bytes_deallocated_in_current_frame;
            }
        }
    }

    // We don't implement alloc_zeroed cuz we don't have a faster way of doing this that's faster than what the default impl would do
    // We might be able to increase performance in the future by zeroing some phys frame in advance and implementing alloc_zeroed ourselves

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        todo!()
    }
}
