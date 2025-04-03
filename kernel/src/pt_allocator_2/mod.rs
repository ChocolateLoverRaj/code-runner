pub mod get_offset_page_table;
pub mod init;
pub mod initial_meta_allocator;
pub mod is_offset_mapped;
pub mod memory_usage_stats;
pub mod meta_frame_allocator;
pub mod pt_allocator_2;
pub mod pt_frame_allocator_2;
pub mod pt_frame_allocator_3;

use alloc::vec::Vec;
use memory_usage_stats::MemoryUsageStats;
use pt_allocator_2::PtAllocator2;
use spinning_top::Spinlock;
use util::{continuous_bool_vec::ContinuousBoolVec, init_later::InitLater};

use crate::not_const_allocator::NotConstAllocator;

pub static PHYS_MEM_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();
pub static MEMORY_USAGE_STATS: InitLater<Spinlock<MemoryUsageStats>> = InitLater::uninit();
pub static KERNEL_ADDRESS_SPACE_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();

#[global_allocator]
static ALLOCATOR: NotConstAllocator<PtAllocator2> = NotConstAllocator::uninit();
