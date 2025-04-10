use alloc::vec::Vec;
use core::ops::DerefMut;

use crate::{continuous_bool_vec::ContinuousBoolVec, testable_allocator::TestableAllocator};

pub mod clone_temp;
pub mod scan_memory;

pub trait GetTrackers {
    fn get_phys_tracker(&mut self) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>>;
    fn get_virt_tracker(&mut self) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>>;
}

pub struct PagingAllocator<F: GetTrackers> {
    get_needed_data: F,
    offset_map_start: usize,
}

impl<F: GetTrackers> PagingAllocator<F> {
    pub const fn new(get_needed_data: F, offset_map_start: usize) -> Self {
        Self {
            get_needed_data,
            offset_map_start,
        }
    }
}

impl<F: GetTrackers> TestableAllocator for PagingAllocator<F> {
    fn allocate(&mut self, layout: core::alloc::Layout) -> Option<core::num::NonZeroUsize> {
        let mut phys_mem = self.get_needed_data.get_phys_tracker();
        let physical_start =
            phys_mem.get_continuous_range_with_alignment(false, layout.size(), layout.align())?;
        phys_mem.set(physical_start..physical_start + layout.size(), true);
        Some((physical_start + self.offset_map_start).try_into().unwrap())
    }

    fn deallocate(&mut self, ptr: core::num::NonZeroUsize, layout: core::alloc::Layout) {
        let mut phys_mem = self.get_needed_data.get_phys_tracker();
        let physical_start = usize::from(ptr) - self.offset_map_start;
        phys_mem.set(physical_start..physical_start + layout.size(), false);
    }
}

#[cfg(test)]
mod tests {
    use core::alloc::Layout;

    use scan_memory::scan_memory;

    use crate::{allocator_test::Memory, mock_memory::MockMemory};

    use super::*;

    #[derive(Debug)]
    struct NeededData {
        phys_mem_tracker: ContinuousBoolVec<Vec<usize>>,
        kernel_address_space_tracker: ContinuousBoolVec<Vec<usize>>,
    }
    impl GetTrackers for NeededData {
        fn get_phys_tracker(&mut self) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>> {
            &mut self.phys_mem_tracker
        }
        fn get_virt_tracker(&mut self) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>> {
            &mut self.kernel_address_space_tracker
        }
    }

    fn get_allocator() -> (MockMemory, PagingAllocator<NeededData>) {
        let (memory, offset_map_start) =
            MockMemory::new_with_offset_map([0x0..0x10_000].into()).unwrap();
        let (phys_mem_tracker, kernel_address_space_tracker) =
            scan_memory(&memory, offset_map_start);

        let needed_data = NeededData {
            phys_mem_tracker,
            kernel_address_space_tracker,
        };
        println!(
            "Data: {:#?}. Offset map start: {}",
            needed_data, offset_map_start
        );
        (memory, PagingAllocator::new(needed_data, offset_map_start))
    }

    #[test]
    pub fn valid_single_byte() {
        let (memory, mut allocator) = get_allocator();
        let layout = Layout::from_size_align(1, 1).unwrap();
        let start = allocator.allocate(layout).unwrap().into();
        println!("ptr: {}", start);
        // Make sure memory is writable
        memory.get_bytes_mut(start..start + layout.size());
    }

    #[test]
    pub fn deallocate() {
        let (_memory, mut allocator) = get_allocator();
        let layout = Layout::from_size_align(1, 1).unwrap();
        let ptr = allocator.allocate(layout).unwrap().into();
        println!("ptr: {}", ptr);
        allocator.deallocate(ptr, layout);
    }
}
