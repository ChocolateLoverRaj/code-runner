use core::{
    cell::RefCell,
    num::NonZeroUsize,
    ops::{DerefMut, Range},
    ptr::NonNull,
};

use alloc::alloc::{AllocError, Allocator};
use limine::response::MemoryMapResponse;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use crate::{
    pt_allocator_2::temp_frame_allocator::TempFrameAllocator, traverse_cr3::PageTableDeepIterator,
    virt_addr_to_number::VirtAddrToNumber,
};

/// An allocator for initializing our allocator
#[derive(Clone)]
pub struct InitialMetaAllocator<'a> {
    pub hhdm_offset: u64,
    pub memory_map_response: &'static MemoryMapResponse,
    pub used_phys_bytes: &'a RefCell<usize>,
}

unsafe impl Allocator for InitialMetaAllocator<'_> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        log::debug!("Allocating with layout: {:?}", layout);
        let get_valid_range = |range: Range<usize>| -> Option<Range<usize>> {
            log::debug!("Checking if range is valid: {:X?}", range);
            let aligned_start =
                round_mult::up(range.start, NonZeroUsize::try_from(layout.align()).unwrap())?;
            if aligned_start + layout.size() <= range.end {
                Some(aligned_start..aligned_start + layout.size())
            } else {
                None
            }
        };
        let mut iter = unsafe { PageTableDeepIterator::new(self.hhdm_offset) };
        let mut free_start = 0x800000000000;
        let valid_range = loop {
            if let Some(mapping) = iter.next() {
                if let Some(valid_range) =
                    get_valid_range(free_start..mapping.virt_start.into_number())
                {
                    break Some(valid_range);
                } else {
                    free_start = mapping.virt_start.into_number() + mapping.len as usize;
                }
            } else {
                break None;
            }
        }
        .or_else(|| get_valid_range(free_start..0xFFFFFFFFFFFF));
        let valid_range = valid_range.ok_or(AllocError)?;

        log::debug!("Using valid range: {:X?}", valid_range);

        // Actually map the memory
        // This is just to make sure we didn't mess up
        assert_eq!(valid_range.start % 0x1000, 0);
        let pages_to_map = Page::<Size4KiB>::containing_address(VirtAddr::new_truncate(
            (valid_range.end - 1) as u64,
        )) - Page::containing_address(VirtAddr::new_truncate(
            valid_range.start as u64,
        )) + 1;
        let mut used_phys_bytes = self.used_phys_bytes.borrow_mut();
        let mut frame_allocator = TempFrameAllocator {
            memory_map_response: self.memory_map_response,
            used_phys_bytes: used_phys_bytes.deref_mut(),
        };
        for i in 0..pages_to_map {
            let mut offset_page_table = unsafe {
                OffsetPageTable::new(
                    {
                        let (active_l4, _cr3_flags) = Cr3::read();
                        let active_l4_pt = (active_l4.start_address().as_u64() + self.hhdm_offset)
                            as *mut PageTable;
                        &mut *active_l4_pt
                    },
                    VirtAddr::new(self.hhdm_offset),
                )
            };
            let page = Page::from_start_address(VirtAddr::new_truncate(valid_range.start as u64))
                .unwrap()
                + i as u64;
            let frame = frame_allocator.allocate_frame().unwrap();
            unsafe {
                offset_page_table.map_to(
                    page,
                    frame,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                    &mut frame_allocator,
                )
            }
            .unwrap()
            .flush();
            log::debug!("Mapped {:?} to {:?}", page, frame);
        }

        Ok(NonNull::from_ref(unsafe {
            core::slice::from_raw_parts_mut(
                VirtAddr::new_truncate(valid_range.start as u64).as_mut_ptr(),
                valid_range.len(),
            )
        }))
    }

    unsafe fn deallocate(&self, _ptr: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        unreachable!("This allocator is not meant to ever deallocate")
    }
}
