use core::{alloc::GlobalAlloc, ops::DerefMut};

use limine::response::MemoryMapResponse;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB, Translate,
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

use super::{is_offset_mapped::is_offset_mapped, KERNEL_ADDRESS_SPACE_TRACKER};

pub struct PtAllocator2 {
    pub(crate) memory_map_response: &'static MemoryMapResponse,
    pub(crate) hhdm_offset: u64,
}

unsafe impl GlobalAlloc for PtAllocator2 {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // First we find the first phys frame
        let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        let mut virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let mut phys_mem_used_by_kernel = PHYS_MEM_USED_BY_KERNEL.try_get().unwrap().lock();

        let first_frame_portion = phys_mem
            .iter()
            .filter(|segment| !segment.value)
            .find_map(|segment| {
                let start = segment.position.next_multiple_of(layout.align());
                let end = (start + 1)
                    .next_multiple_of(0x1000)
                    .min(start + layout.size());
                if end <= segment.position + segment.len {
                    Some(start..end)
                } else {
                    None
                }
            })
            .unwrap();
        log::info!(
            "Using phys range: {:X?}. Layout: {:#?}",
            first_frame_portion,
            layout,
        );
        phys_mem.set(first_frame_portion.clone(), true);
        *phys_mem_used_by_kernel += first_frame_portion.len();

        // If the entire layout is in a single phys frame, we can just use a pointer to an offset-mapped page
        let ptr = if first_frame_portion.len() == layout.size() {
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
            log::debug!("Using virt range: {:X?}", virt_range);
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
                *phys_mem_used_by_kernel += map_len;
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
        };
        log::info!("alloc: {:?}", ptr);
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        log::info!("dealloc: {:?}", ptr);
        let is_offset_mapped = is_offset_mapped(
            self.memory_map_response,
            self.hhdm_offset,
            VirtAddr::from_ptr(ptr),
        );

        let mut phys_tracker = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        let mut phys_mem_used_by_kernel = PHYS_MEM_USED_BY_KERNEL.try_get().unwrap().lock();
        if is_offset_mapped {
            // Just mark phys as unused, that's all
            let range = {
                let start = (VirtAddr::from_ptr(ptr) - self.hhdm_offset).into_number();
                start..start + layout.size()
            };
            log::debug!("Deallocating HDDM - Marking phys: {:X?} as usable", range);
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
                    log::debug!("Marking virt as usable: {:X?}", range);
                    virt_tracker.set(range, false);
                }
                bytes_deallocated += bytes_deallocated_in_current_frame;
            }
        }
        *phys_mem_used_by_kernel -= layout.size();
    }

    // We don't implement alloc_zeroed cuz we don't have a faster way of doing this that's faster than what the default impl would do
    // We might be able to increase performance in the future by zeroing some phys frame in advance and implementing alloc_zeroed ourselves

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        log::info!(
            "realloc: {:?}. layout: {:?}. new size: {:?}",
            ptr,
            layout,
            new_size
        );
        // We can try growing the existing virt address range (to the left or to the right)
        // But I don't feel like it so I'm not going to 😎. Also the way this allocator is designed, checking if we can grow left or right might make performance worse

        // Get new virt range
        let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        let mut virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let mut phys_mem_used_by_kernel = PHYS_MEM_USED_BY_KERNEL.try_get().unwrap().lock();

        if new_size > layout.size() {
            let current_start_number = VirtAddr::from_ptr(ptr).into_number();
            log::info!(
                "Current start number: 0x{:X}. Layout size: 0x{:X}",
                current_start_number,
                layout.size()
            );
            let current_page_count = {
                (current_start_number + (layout.size() - 1)).div_ceil(0x1000)
                    - current_start_number.div_floor(0x1000)
            };
            let new_page_count = {
                let first_page = Page::<Size4KiB>::containing_address(VirtAddr::from_ptr(ptr));
                let last_page = Page::<Size4KiB>::containing_address(
                    VirtAddr::from_ptr(ptr) + layout.size() as u64 - 1,
                ) + 1;
                (last_page - first_page) as usize
            };
            let new_virt_start_page_addr = virt_mem
                .get_continuous_range_with_alignment(false, new_page_count * 0x1000, 0x1000)
                .unwrap();
            let current_start_page = Page::<Size4KiB>::containing_address(VirtAddr::from_ptr(ptr));
            let new_start_page = Page::<Size4KiB>::containing_address(VirtAddr::new_truncate(
                new_virt_start_page_addr as u64,
            ));
            let new_virt_start = new_virt_start_page_addr + (ptr as usize % 0x1000);
            let mut offset_page_table = get_offset_page_table(self.hhdm_offset);
            let mut frame_allocator = PtFrameAllocator {
                phys: &mut phys_mem,
            };

            let last_frame_is_full = (current_start_number + layout.size()) % 0x1000 == 0;
            let current_full_page_count = if last_frame_is_full {
                current_page_count
            } else {
                current_page_count - 1
            };
            let is_offset_mapped = is_offset_mapped(
                self.memory_map_response,
                self.hhdm_offset,
                VirtAddr::from_ptr(ptr),
            );

            // Switch existing mappings
            for page_offset in 0..current_full_page_count {
                // No need to flush because the unmapped memory won't be used anyway
                let frame = if is_offset_mapped {
                    PhysFrame::from_start_address(
                        offset_page_table
                            .translate_addr(
                                (current_start_page + page_offset as u64).start_address(),
                            )
                            .unwrap(),
                    )
                    .unwrap()
                } else {
                    offset_page_table
                        .unmap(current_start_page + page_offset as u64)
                        .unwrap()
                        .0
                };
                unsafe {
                    offset_page_table.map_to(
                        new_start_page + page_offset as u64,
                        frame,
                        PageTableFlags::PRESENT,
                        &mut frame_allocator,
                    )
                }
                .unwrap()
                .flush();
            }

            if !last_frame_is_full {
                // TODO: For better performance, try to extend the last phys frame

                // Find a new phys frame and map to it
                log::info!("Phys mem: {:#?}", phys_mem);
                let mut frame_allocator = PtFrameAllocator {
                    phys: &mut phys_mem,
                };
                let frame = frame_allocator.allocate_frame().unwrap();
                log::info!("Frame: {:?}", frame);
                unsafe {
                    offset_page_table.map_to(
                        new_start_page + current_full_page_count as u64,
                        frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut frame_allocator,
                    )
                }
                .unwrap()
                .flush();

                // Copy existing memory
                // TODO: May improve performance if we don't copy all 4KiB
                let current_page = current_start_page + current_full_page_count as u64;
                let current_frame_ptr = VirtAddr::new_truncate(
                    offset_page_table
                        .translate_addr(current_page.start_address())
                        .unwrap()
                        .as_u64()
                        + self.hhdm_offset,
                )
                .as_mut_ptr::<u8>();
                let new_frame_ptr =
                    VirtAddr::new_truncate(frame.start_address().as_u64() + self.hhdm_offset)
                        .as_mut_ptr();
                log::info!(
                    "Copying from {:?} to {:?}. Current page count: {}. Current full page count: {}, is offset mapped: {}",
                    current_frame_ptr,
                    new_frame_ptr,
                    current_page_count,
                    current_full_page_count,
                    is_offset_mapped
                );
                unsafe { core::ptr::copy_nonoverlapping(current_frame_ptr, new_frame_ptr, 0x1000) };

                // Unmap old
                if !is_offset_mapped {
                    offset_page_table.unmap(current_page).unwrap();
                }
            }

            let mut frame_allocator = PtFrameAllocator {
                phys: &mut phys_mem,
            };
            for page_offset in current_page_count..new_page_count {
                // Allocate new frames
                let frame = frame_allocator.allocate_frame().unwrap();
                unsafe {
                    offset_page_table.map_to(
                        new_start_page + page_offset as u64,
                        frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut frame_allocator,
                    )
                }
                .unwrap()
                .flush();
            }

            virt_mem.set(new_virt_start..new_virt_start + new_size, true);

            VirtAddr::new_truncate(new_virt_start as u64).as_mut_ptr()
        } else {
            todo!()
        }
    }
}
