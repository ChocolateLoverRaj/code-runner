// pub mod init_2;
pub mod initial_meta_allocator;
pub mod is_offset_mapped;
// pub mod locked_allocator;
pub mod locked_mem_trackers;
pub mod memory_usage_stats;
pub mod meta_frame_allocator;
pub mod pre_reserved_pages;
pub mod pt_allocator_2;
pub mod pt_frame_allocator_2;
pub mod pt_frame_allocator_3;
pub mod reuse_static_pages;

use alloc::vec::Vec;
// use locked_allocator::LockedAllocator;
use locked_mem_trackers::LockedMemTrackers;
use memory_usage_stats::MemoryUsageStats;
use spinning_top::Spinlock;
use util::{
    continuous_bool_vec::ContinuousBoolVec, init_later::InitLater,
    paging_allocator::PagingAllocator, static_allocator::StaticAllocator,
    x86_64_allocator::X86_64Allocator,
};

pub static PHYS_MEM_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();
pub static MEMORY_USAGE_STATS: InitLater<Spinlock<MemoryUsageStats>> = InitLater::uninit();
pub static KERNEL_ADDRESS_SPACE_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();

// static LOCKED_ALLOCATOR: InitLater<Spinlock<PagingAllocator<LockedMemTrackers>>> =
//     InitLater::uninit();

// basically a todo!() allocator
#[global_allocator]
static ALLOCATOR: StaticAllocator = StaticAllocator;
