use limine::response::MemoryMapResponse;
use spinning_top::Spinlock;
use util::{
    paging_allocator::{clone_temp::clone_temp, scan_memory::scan_memory, PagingAllocator},
    x86_memory::x86_memory::X86Memory,
};

use crate::hhdm_offset::HhdmOffset;

use super::{
    locked_mem_trackers::LockedMemTrackers, memory_usage_stats::MemoryUsageStats,
    KERNEL_ADDRESS_SPACE_TRACKER, LOCKED_ALLOCATOR, MEMORY_USAGE_STATS, PHYS_MEM_TRACKER,
};

pub fn init_2(memory: &'static MemoryMapResponse, hhdm_offset: HhdmOffset) {
    let (phys_mem_tracker, kernel_address_space_tracker) =
        scan_memory(&X86Memory::new(memory), u64::from(hhdm_offset) as usize);
    let (phys_mem_tracker, kernel_address_space_tracker) =
        clone_temp(phys_mem_tracker, kernel_address_space_tracker);
    PHYS_MEM_TRACKER
        .try_init(Spinlock::new(phys_mem_tracker))
        .unwrap();
    KERNEL_ADDRESS_SPACE_TRACKER
        .try_init(Spinlock::new(kernel_address_space_tracker))
        .unwrap();
    let testable_allocator =
        PagingAllocator::new(LockedMemTrackers, u64::from(hhdm_offset) as usize);
    LOCKED_ALLOCATOR
        .try_init(Spinlock::new(testable_allocator))
        .unwrap();
    MEMORY_USAGE_STATS
        .try_init(Spinlock::new(MemoryUsageStats::default()))
        .unwrap();
}
