use alloc::vec::Vec;
use core::{num::NonZeroUsize, ops::DerefMut};

use crate::{
    continuous_bool_vec::ContinuousBoolVec,
    testable_allocator::{AllocationResult, AllocatorContext, TestableAllocator},
};

pub mod clone_temp;
pub mod scan_memory;

pub trait GetTrackers {
    fn get_phys_tracker(&mut self) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>>;
    fn get_virt_tracker(&mut self) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>>;
}

pub struct PagingAllocator {
    offset_map_start: usize,
}

impl PagingAllocator {
    pub const fn new(offset_map_start: usize) -> Self {
        Self { offset_map_start }
    }
}

impl TestableAllocator for PagingAllocator {
    fn allocate(
        &mut self,
        layout: core::alloc::Layout,
        context: AllocatorContext<'_>,
    ) -> Option<AllocationResult<NonZeroUsize>> {
        // Every allocation could increase phys tracker len by up to 2
        // Every deallocation could increase phys tracker len by up to 2
        // Every reallocation could cause an allocation and a deallocation, so it could increase phy stracker by up to 4
        if context.is_meta_data_operation {
            // Must have len of 2
            if context.physical_memory_tracker.len_vec.capacity()
                - context.physical_memory_tracker.len_vec.len()
                < 2
            {
                unreachable!()
            }
        } else {
            let spare_capacity = context.physical_memory_tracker.len_vec.capacity()
                - context.physical_memory_tracker.len_vec.len();
            if spare_capacity < 6 {
                return Some(AllocationResult::NeedsGrowPhys(6 - spare_capacity));
            }
        }

        let physical_start = context
            .physical_memory_tracker
            .get_continuous_range_with_alignment(false, layout.size(), layout.align())?;
        context
            .physical_memory_tracker
            .set(physical_start..physical_start + layout.size(), true);
        Some(AllocationResult::Done(
            (physical_start + self.offset_map_start).try_into().unwrap(),
        ))
    }

    fn deallocate(
        &mut self,
        ptr: core::num::NonZeroUsize,
        layout: core::alloc::Layout,
        context: AllocatorContext<'_>,
    ) -> AllocationResult<()> {
        if context.is_meta_data_operation {
            // Must have len of 2
            if context.physical_memory_tracker.len_vec.capacity()
                - context.physical_memory_tracker.len_vec.len()
                < 2
            {
                unreachable!()
            }
        } else {
            let spare_capacity = context.physical_memory_tracker.len_vec.capacity()
                - context.physical_memory_tracker.len_vec.len();
            if spare_capacity < 6 {
                return AllocationResult::NeedsGrowPhys(6 - spare_capacity);
            }
        }

        let physical_start = usize::from(ptr) - self.offset_map_start;
        context
            .physical_memory_tracker
            .set(physical_start..physical_start + layout.size(), false);
        AllocationResult::Done(())
    }
}

#[cfg(test)]
mod tests {
    use core::alloc::Layout;

    use scan_memory::scan_memory;

    use crate::{allocator_test::Memory, mock_memory::MockMemory};

    use super::*;

    fn get_allocator() -> (
        MockMemory,
        PagingAllocator,
        ContinuousBoolVec<Vec<usize>>,
        ContinuousBoolVec<Vec<usize>>,
    ) {
        let (memory, offset_map_start) =
            MockMemory::new_with_offset_map([0x0..0x10_000].into()).unwrap();
        let (phys_mem_tracker, kernel_address_space_tracker) =
            scan_memory(&memory, offset_map_start);
        (
            memory,
            PagingAllocator::new(offset_map_start),
            phys_mem_tracker,
            kernel_address_space_tracker,
        )
    }

    #[test]
    pub fn valid_single_byte() {
        let (memory, mut allocator, mut physical_memory, mut virtual_memory) = get_allocator();
        let layout = Layout::from_size_align(1, 1).unwrap();
        let start = allocator
            .allocate(
                layout,
                AllocatorContext {
                    physical_memory_tracker: &mut physical_memory,
                    virtual_memory_tracker: &mut virtual_memory,
                    is_meta_data_operation: false,
                },
            )
            .unwrap()
            .unwrap_done()
            .into();
        println!("ptr: {}", start);
        // Make sure memory is writable
        memory.get_bytes_mut(start..start + layout.size());
    }

    #[test]
    pub fn deallocate() {
        let (_memory, mut allocator, mut physical_memory, mut virtual_memory) = get_allocator();
        let layout = Layout::from_size_align(1, 1).unwrap();
        let ptr = allocator
            .allocate(
                layout,
                AllocatorContext {
                    physical_memory_tracker: &mut physical_memory,
                    virtual_memory_tracker: &mut virtual_memory,
                    is_meta_data_operation: false,
                },
            )
            .unwrap()
            .unwrap_done()
            .into();
        println!("ptr: {}", ptr);
        allocator
            .deallocate(
                ptr,
                layout,
                AllocatorContext {
                    physical_memory_tracker: &mut physical_memory,
                    virtual_memory_tracker: &mut virtual_memory,
                    is_meta_data_operation: false,
                },
            )
            .unwrap_done();
    }
}
