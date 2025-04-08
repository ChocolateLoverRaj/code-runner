use core::alloc::GlobalAlloc;

use limine::response::MemoryMapResponse;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};

use crate::{
    hhdm_offset::HhdmOffset,
    pt_allocator_2::{
        get_offset_page_table::get_offset_page_table, pt_frame_allocator_3::PtFrameAllocator3,
        MEMORY_USAGE_STATS, PHYS_MEM_TRACKER,
    },
    virt_addr_to_number::VirtAddrToNumber,
};

use super::{is_offset_mapped::is_offset_mapped, KERNEL_ADDRESS_SPACE_TRACKER};

pub struct PtAllocator2 {
    pub(crate) memory_map_response: &'static MemoryMapResponse,
    pub(crate) hhdm_offset: HhdmOffset,
}

unsafe impl GlobalAlloc for PtAllocator2 {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let mut memory_usage_stats = MEMORY_USAGE_STATS.try_get().unwrap().lock();

        // First we find the first phys frame
        let first_frame_portion = {
            let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
            let first_frame_portion = phys_mem
                .iter()
                .filter(|segment| !segment.value)
                .find_map(|segment| {
                    let start = segment.position.next_multiple_of(layout.align());
                    // This is the end of the frame, or the end of the data, depending on which one is smaller
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
            phys_mem.set(first_frame_portion.clone(), true);
            memory_usage_stats.global_allocations += first_frame_portion.len();
            first_frame_portion
        };

        // If the entire layout is in a single phys frame, we can just use a pointer to an offset-mapped page
        let ptr = if first_frame_portion.len() == layout.size() {
            VirtAddr::new_truncate(first_frame_portion.start as u64 + u64::from(self.hhdm_offset))
                .as_mut_ptr()
        } else {
            // Find continuous virt range (virt must be continuous, phys only has to be made of continuous 4KiB chunks)
            let page_count = {
                let first_frame_number = first_frame_portion.start.div_floor(0x1000);
                let last_frame_number =
                    (first_frame_portion.start + (layout.size() - 1)).div_ceil(0x1000);
                last_frame_number - first_frame_number
            };
            let virt_pages_start = virt_mem
                .get_continuous_range_with_alignment(false, 0x1000 * page_count, 0x1000)
                .unwrap();
            let data_virt_range = {
                let start = virt_pages_start + first_frame_portion.start % 0x1000;
                start..start + layout.size()
            };
            // log::debug!("Using virt range: {:X?}", data_virt_range);
            virt_mem.set(data_virt_range.clone(), true);

            // Map the first frame
            let mut offset_page_table = get_offset_page_table(self.hhdm_offset);
            let start_page = Page::<Size4KiB>::from_start_address(VirtAddr::new_truncate(
                virt_pages_start as u64,
            ))
            .unwrap();
            {
                let mut frame_allocator = PtFrameAllocator3 {
                    f: |_| {
                        memory_usage_stats.page_tables += 0x1000;
                    },
                };
                let first_frame =
                    PhysFrame::containing_address(PhysAddr::new(first_frame_portion.start as u64));
                unsafe {
                    offset_page_table.map_to(
                        start_page,
                        first_frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut frame_allocator,
                    )
                }
                .unwrap()
                .flush();
            }

            // Map rest of memory
            let mut remaining_to_map = layout.size() - first_frame_portion.len();
            let mut page_offset = 1;
            loop {
                if remaining_to_map == 0 {
                    break;
                }
                // The last frame might not extend to the frame boundary, so we don't need the whole frame in that case
                let bytes_to_map = remaining_to_map.min(0x1000);
                let phys_frame = PtFrameAllocator3 {
                    f: |_| {
                        memory_usage_stats.global_allocations += 0x1000;
                    },
                }
                .allocate_frame()
                .unwrap();
                let mut frame_allocator = PtFrameAllocator3 {
                    f: |_| {
                        memory_usage_stats.page_tables += 0x1000;
                    },
                };
                unsafe {
                    offset_page_table.map_to(
                        start_page + page_offset,
                        phys_frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut frame_allocator,
                    )
                }
                .unwrap()
                .flush();
                remaining_to_map -= bytes_to_map;
                page_offset += 1;
            }

            VirtAddr::new_truncate(data_virt_range.start as u64).as_mut_ptr()
        };

        // log::info!("alloc: {:?}. layout: {:?}", ptr, layout);
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        // log::info!("dealloc: {:?}. layout: {:?}", ptr, layout);

        let mut phys_tracker = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        let mut virt_tracker = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let mut memory_usage = MEMORY_USAGE_STATS.try_get().unwrap().lock();

        let is_offset_mapped = is_offset_mapped(
            self.memory_map_response,
            self.hhdm_offset,
            VirtAddr::from_ptr(ptr),
        );

        if is_offset_mapped {
            // Just mark phys as unused. No need to change page mappings.
            let phys_range = {
                let phys_start =
                    (VirtAddr::from_ptr(ptr) - u64::from(self.hhdm_offset)).into_number();
                phys_start..phys_start + layout.size()
            };
            // log::debug!(
            //     "Deallocating HDDM - Marking phys: {:X?} as usable",
            //     phys_range
            // );
            phys_tracker.set(phys_range, false);
        } else {
            // We decrement this because it's normal in programming to forget what we've already done, so we "forget" by decrementing
            // If we incremented from 0 to layout.size() that would be like remembering what we've already done
            let mut bytes_left_to_deallocate = layout.size();
            let mut offset_page_table = get_offset_page_table(self.hhdm_offset);
            loop {
                if bytes_left_to_deallocate == 0 {
                    break;
                }
                let bytes_deallocated = layout.size() - bytes_left_to_deallocate;
                let virt = VirtAddr::from_ptr(ptr) + bytes_deallocated as u64;
                // let phys = offset_page_table.translate_addr(virt).unwrap();
                // If this is the first page, then it might not be aligned
                let actual_virt_start_offset_in_page = virt.as_u64() as usize % 0x1000;
                let bytes_deallocated_in_current_frame =
                    (0x1000 - actual_virt_start_offset_in_page).min(bytes_left_to_deallocate);
                // No need to flush because we won't be using the pointer
                let page_to_unmap = Page::<Size4KiB>::containing_address(virt);
                let r = offset_page_table.translate_page(page_to_unmap);
                // log::info!("Unmapping page: {:?}. Translation: {:?}", page_to_unmap, r,);
                let (phys_frame, _flush) = offset_page_table.unmap(page_to_unmap).unwrap();
                {
                    let phys_start = (phys_frame.start_address()
                        + actual_virt_start_offset_in_page as u64)
                        .as_u64() as usize;
                    let range = phys_start..phys_start + bytes_deallocated_in_current_frame;
                    // log::debug!("Marking phys as usable: {:X?}", range);
                    phys_tracker.set(range, false);
                }
                bytes_left_to_deallocate -= bytes_deallocated_in_current_frame;
            }
            // Mark virt range as usable
            let ptr_number_range = {
                let ptr_start_number = VirtAddr::from_ptr(ptr).into_number();
                ptr_start_number..ptr_start_number + layout.size()
            };
            // log::debug!("Marking virt as usable: {:X?}", ptr_number_range);
            virt_tracker.set(ptr_number_range, false);
        }

        // We deallocated the entire layout in physical memory
        memory_usage.global_allocations -= layout.size();
    }

    // We don't implement alloc_zeroed cuz we don't have a faster way of doing this that's faster than what the default impl would do
    // We might be able to increase performance in the future by zeroing some phys frame in advance and implementing alloc_zeroed ourselves

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let mut virt_mem = KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock();
        let mut memory_usage = MEMORY_USAGE_STATS.try_get().unwrap().lock();

        // We can try growing the existing virt address range (to the left or to the right)
        // But I don't feel like it so I'm not going to 😎. Also the way this allocator is designed, checking if we can grow left or right might make performance worse

        let mut offset_page_table = get_offset_page_table(self.hhdm_offset);
        let is_offset_mapped = is_offset_mapped(
            self.memory_map_response,
            self.hhdm_offset,
            VirtAddr::from_ptr(ptr),
        );
        let new_ptr = if new_size > layout.size() {
            let current_start_number = VirtAddr::from_ptr(ptr).into_number();
            // log::debug!(
            //     "Current start number: 0x{:X}. Layout size: 0x{:X}",
            //     current_start_number,
            //     layout.size()
            // );
            let current_page_count = {
                (current_start_number + layout.size()).div_ceil(0x1000)
                    - current_start_number.div_floor(0x1000)
            };
            let new_page_count = (current_start_number + new_size).div_ceil(0x1000)
                - current_start_number.div_floor(0x1000);
            let new_virt_start_page_addr = virt_mem
                .get_continuous_range_with_alignment(false, new_page_count * 0x1000, 0x1000)
                .unwrap();
            virt_mem.set(
                new_virt_start_page_addr..new_virt_start_page_addr + new_page_count * 0x1000,
                true,
            );
            let current_start_page = Page::<Size4KiB>::containing_address(VirtAddr::from_ptr(ptr));
            let new_start_page = Page::<Size4KiB>::containing_address(VirtAddr::new_truncate(
                new_virt_start_page_addr as u64,
            ));
            let new_virt_start = new_virt_start_page_addr + (ptr as usize % 0x1000);

            let last_partial_frame_bytes = (current_start_number + layout.size()) % 0x1000;
            let last_frame_is_full = last_partial_frame_bytes == 0;
            // log::debug!(
            //     "Current page count: {}. New page count: {}. Last frame is full?: {}",
            //     current_page_count,
            //     new_page_count,
            //     last_frame_is_full
            // );
            let current_full_page_count = if last_frame_is_full {
                current_page_count
            } else {
                current_page_count - 1
            };

            // Switch existing mappings
            let mut frame_allocator = PtFrameAllocator3 {
                f: |_frame| {
                    memory_usage.page_tables += 0x1000;
                },
            };
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
                    let page = current_start_page + page_offset as u64;
                    // log::info!("Unmapping page: {:?}", page);
                    offset_page_table.unmap(page).unwrap().0
                };
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

            if !last_frame_is_full {
                // TODO: For better performance, try to extend the last phys frame

                // Find a new phys frame and map to it
                let frame = PtFrameAllocator3 { f: |_| {} }.allocate_frame().unwrap();
                // log::debug!("Frame: {:?}", frame);
                let mut frame_allocator = PtFrameAllocator3 {
                    f: |_frame| {
                        memory_usage.page_tables += 0x1000;
                    },
                };
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
                let current_page = current_start_page + current_full_page_count as u64;
                let current_frame_ptr = VirtAddr::new_truncate(
                    offset_page_table
                        .translate_addr(current_page.start_address())
                        .unwrap()
                        .as_u64()
                        + u64::from(self.hhdm_offset),
                )
                .as_mut_ptr::<u8>();
                let new_frame_ptr = VirtAddr::new_truncate(
                    frame.start_address().as_u64() + u64::from(self.hhdm_offset),
                )
                .as_mut_ptr();
                // log::info!(
                //     "Copying from {:?} to {:?}. Current page count: {}. Current full page count: {}, is offset mapped: {}. copying bytes: {}",
                //     current_frame_ptr,
                //     new_frame_ptr,
                //     current_page_count,
                //     current_full_page_count,
                //     is_offset_mapped,
                //     last_partial_frame_bytes
                // );
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        current_frame_ptr,
                        new_frame_ptr,
                        last_partial_frame_bytes,
                    )
                };

                // Unmap old
                if !is_offset_mapped {
                    // log::info!("Unmapping page: {:?}", current_page);
                    let _ = offset_page_table.unmap(current_page).unwrap();
                }
            }

            for page_offset in current_page_count..new_page_count {
                // Allocate new frames
                let frame = PtFrameAllocator3 { f: |_| {} }.allocate_frame().unwrap();
                let mut frame_allocator = PtFrameAllocator3 {
                    f: |_frame| {
                        memory_usage.page_tables += 0x1000;
                    },
                };
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

            VirtAddr::new_truncate(new_virt_start as u64).as_mut_ptr()
        } else {
            let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
            let start_virt_addr = VirtAddr::from_ptr(ptr);
            let ptr_number = start_virt_addr.into_number();
            let pages_to_unmap = (ptr_number + layout.size()).div_ceil(0x1000)
                - (ptr_number + new_size).div_ceil(0x1000);
            let start_page = Page::<Size4KiB>::containing_address(start_virt_addr);
            // Mark the phys mem of the first frame unused
            if ptr_number + new_size < ptr_number.next_multiple_of(0x1000) {
                let start_phys_addr = offset_page_table.translate_addr(start_virt_addr).unwrap();
                let range = {
                    let start_number = start_phys_addr.as_u64() as usize;
                    let start = start_number + new_size;
                    let end =
                        (start_number + layout.size()).min(start_number.next_multiple_of(0x1000));
                    start..end
                };
                // phys_mem.set(range, false);
            }
            // Unmap every other page
            let first_page_to_unmap = start_page + 1;
            for page in first_page_to_unmap..first_page_to_unmap + pages_to_unmap as u64 {
                let (phys_frame, _flush) = offset_page_table.unmap(page).unwrap();
                // let range = {
                //     let start = phys_frame.start_address().as_u64() as usize;
                //     start..start + (ptr_number + layout.size() - ptr_number.next_multiple_of(0x1000) - 0x1000 * )
                // };
                // phys_mem.set(range, false);
            }

            // Mark virt range as unused, if it's not offset mapped
            if !is_offset_mapped {
                let range = {
                    let start = VirtAddr::from_ptr(ptr).into_number() + new_size;
                    let end = VirtAddr::from_ptr(ptr).into_number() + layout.size();
                    start..end
                };
                virt_mem.set(range, false);
            }

            // We did not change the location of the pointer
            ptr
        };

        memory_usage.global_allocations -= layout.size();
        memory_usage.global_allocations += new_size;

        // log::info!(
        //     "realloc: {:?}. layout: {:?}. new size: {:?}. new ptr: {:?}",
        //     ptr,
        //     layout,
        //     new_size,
        //     new_ptr
        // );
        new_ptr
    }
}
