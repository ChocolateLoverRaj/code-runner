pub mod get_offset_page_table;
pub mod init;
pub mod initial_meta_allocator;
pub mod is_offset_mapped;
pub mod meta_frame_allocator;
pub mod pt_allocator_2;
pub mod pt_frame_allocator;
pub mod pt_frame_allocator_2;

use core::ops::Range;

use alloc::vec::Vec;
use pt_allocator_2::PtAllocator2;
use spinning_top::Spinlock;
use util::{continuous_bool_vec::ContinuousBoolVec, init_later::InitLater};

use crate::not_const_allocator::NotConstAllocator;

pub static PHYS_MEM_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();
pub static PHYS_MEM_USED_BY_KERNEL: InitLater<Spinlock<usize>> = InitLater::uninit();
pub static KERNEL_ADDRESS_SPACE_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();

fn align_range(range: Range<usize>, alignment: usize) -> Range<usize> {
    range.start.div_floor(alignment) * alignment..range.end.div_ceil(alignment) * alignment
}

#[global_allocator]
static ALLOCATOR: NotConstAllocator<PtAllocator2> = NotConstAllocator::uninit();
