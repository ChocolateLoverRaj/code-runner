use alloc::vec::Vec;
use core::{alloc::Layout, fmt::Debug, num::NonZeroUsize};

use crate::continuous_bool_vec::ContinuousBoolVec;

#[derive(Debug)]
pub enum AllocationResult<T> {
    Done(T),
    NeedsGrowPhys(usize),
    NeedsGrowVirt(usize),
}

impl<T: Debug> AllocationResult<T> {
    pub fn unwrap_done(self) -> T {
        match self {
            AllocationResult::Done(value) => value,
            other => panic!("{:#?}", other),
        }
    }
}

pub struct AllocatorContext<'a> {
    pub physical_memory_tracker: &'a mut ContinuousBoolVec<Vec<usize>>,
    pub virtual_memory_tracker: &'a mut ContinuousBoolVec<Vec<usize>>,
    pub is_meta_data_operation: bool,
}

pub trait TestableAllocator {
    fn allocate(
        &mut self,
        layout: Layout,
        context: AllocatorContext<'_>,
    ) -> Option<AllocationResult<NonZeroUsize>>;

    fn deallocate(
        &mut self,
        ptr: NonZeroUsize,
        layout: Layout,
        context: AllocatorContext<'_>,
    ) -> AllocationResult<()>;
}
