use core::ops::Range;

use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageSize,
        PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use crate::{find_used_virt_addrs::find_used_virt_addrs, virt_mem_allocator::VirtMemAllocator};

pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const N: usize = 512;

type AllocatorPageSize = Size4KiB;

pub fn init_heap(
    mapper: &mut OffsetPageTable<'static>,
    frame_allocator: &mut impl FrameAllocator<AllocatorPageSize>,
) -> Result<heapless::Vec<Range<VirtAddr>, N>, MapToError<AllocatorPageSize>> {
    let mut ranges = find_used_virt_addrs(mapper.level_4_table(), mapper.phys_offset());
    let heap_start = ranges
        .allocate_pages::<AllocatorPageSize>((HEAP_SIZE as u64).div_ceil(AllocatorPageSize::SIZE))
        .unwrap();
    let page_range = {
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page_exclusive = Page::containing_address(heap_start + HEAP_SIZE as u64);
        Page::range(heap_start_page, heap_end_page_exclusive)
    };
    log::debug!(
        "Pages used for heap: {:?}",
        page_range.start..page_range.end
    );

    let mut flush = None;
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        flush = Some(unsafe { mapper.map_to(page, frame, flags, frame_allocator)? });
    }
    flush.unwrap().flush();

    unsafe {
        ALLOCATOR
            .lock()
            .init(heap_start.as_ptr::<u8>() as *mut u8, HEAP_SIZE);
    }

    Ok(ranges)
}
