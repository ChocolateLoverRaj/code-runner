use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use crate::find_used_virt_addrs::find_used_virt_addrs;

pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(
    mapper: &mut OffsetPageTable<'static>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let ranges = find_used_virt_addrs::<512>(mapper.level_4_table(), mapper.phys_offset());
    let heap_start = {
        // 0 cannot be used since that's reserved for a null pointer
        let mut heap_start = VirtAddr::new(1);
        let mut iter = ranges.iter();
        loop {
            match iter.next() {
                Some(range) => {
                    if heap_start + HEAP_SIZE as u64 <= range.start {
                        break Some(heap_start);
                    }
                    heap_start = range.end;
                }
                None => {
                    if heap_start.as_u64() + HEAP_SIZE as u64 <= (1 << 48) {
                        break Some(heap_start);
                    } else {
                        break None;
                    }
                }
            }
        }
    }
    .unwrap();
    log::info!("Ranges: {:?}. Heap start: {:?}", ranges, heap_start);

    let page_range = {
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    unsafe {
        ALLOCATOR
            .lock()
            .init(heap_start.as_ptr::<u8>() as *mut u8, HEAP_SIZE);
    }

    Ok(())
}
