use anyhow::anyhow;
use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, PageSize, PageTableFlags,
        Size4KiB,
    },
    VirtAddr,
};

use crate::{
    find_used_virt_addrs::find_used_virt_addrs,
    jmp_to_elf::FLEXIBLE_VIRT_MEM_START,
    virt_mem_allocator::{VirtMemAllocator, VirtMemTracker},
};

pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const N: usize = 512;

type AllocatorPageSize = Size4KiB;

pub fn init_heap(
    mapper: &mut OffsetPageTable<'static>,
    frame_allocator: &mut impl FrameAllocator<AllocatorPageSize>,
) -> anyhow::Result<VirtMemTracker> {
    let mut virt_mem_tracker = VirtMemTracker::new(
        VirtAddr::new(FLEXIBLE_VIRT_MEM_START)..VirtAddr::new(0xFFFFFFFFFFFFFFFF),
    );
    log::info!("Finding used virt addrs");
    find_used_virt_addrs(
        mapper.level_4_table(),
        mapper.phys_offset(),
        &mut virt_mem_tracker,
    );
    log::info!("Virt mem tracker: {:#?}", virt_mem_tracker);
    let page_count = (HEAP_SIZE as u64).div_ceil(Size4KiB::SIZE);
    let heap_start = virt_mem_tracker
        .allocate_pages::<Size4KiB>(page_count)
        .ok_or(anyhow!("Failed to allocate bytes for heap"))?;

    let page_range = heap_start..heap_start + page_count;
    log::info!("Pages used for heap: {:?}", page_range);

    let mut flush = None;
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(anyhow!("Failed to allocate frame"))?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        flush = Some(unsafe {
            mapper
                .map_to(page, frame, flags, frame_allocator)
                .map_err(|_e| anyhow!("Failed to map frame"))?
        });
    }
    if let Some(flush) = flush {
        flush.flush()
    };

    unsafe {
        ALLOCATOR.lock().init(
            heap_start.start_address().as_mut_ptr(),
            (page_count * Size4KiB::SIZE) as usize,
        );
    }

    Ok(virt_mem_tracker)
}
