use alloc::vec::Vec;
use core::{borrow::BorrowMut, cell::RefCell, ops::DerefMut};

use crate::{
    allocator_test::{Memory, TestableAllocator},
    bump_allocator::BumpAllocator,
    continuous_bool_vec::ContinuousBoolVec,
    traverse_page_tables::PageTablesTraverser,
};

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
}

#[cfg(not(test))]
type TempAllocator = crate::static_allocator::StaticAllocator;
#[cfg(test)]
type TempAllocator = std::alloc::Global;

pub fn scan_memory<'a, T: Memory>(
    memory: &'a T,
    offset_map_start: usize,
) -> (
    ContinuousBoolVec<Vec<usize, TempAllocator>>,
    ContinuousBoolVec<Vec<usize, TempAllocator>>,
) {
    // Calculate the number of pages. These pages could be >4KiB, and that's okay
    const PAGE_SIZE: usize = 0x1000;
    const ENTRY_SIZE: usize = size_of::<u64>() * 2;
    const ENTRIES_PER_PAGE: usize = PAGE_SIZE / ENTRY_SIZE;

    // Calculate the max vec size needed (for both phys and virt)
    // For every 256 entries (each entry up to 2 x u64, so 16B), we will need 1 page
    // For every page, there will be up to 4 phys frames and 1 page used
    let up_to_n_phys_entries = memory.usable_physical_memory_regions().count();
    let up_to_n_virt_entries = {
        let mut disconnected_ranges = 0_usize;
        let mut end = None;
        PageTablesTraverser::new(
            memory,
            memory.get_current_top_level_page_table(),
            offset_map_start,
        )
        .for_each(|mapping| {
            // #[cfg(test)]
            // println!("Mapping: {:?}", mapping);
            if end != Some(mapping.virt_start) {
                disconnected_ranges += 1;
                end = Some(mapping.virt_start + mapping.len);
            } else {
                end = Some(mapping.virt_start + mapping.len);
            }
        });
        disconnected_ranges
    };

    #[cfg(test)]
    println!(
        "Phys {:?}. Virt: {:?}",
        up_to_n_phys_entries, up_to_n_virt_entries
    );

    // See `entries.md` for the math behind this
    let up_to_total_pages_used = up_to_n_phys_entries.div_ceil(ENTRIES_PER_PAGE)
        + up_to_n_virt_entries.div_ceil(ENTRIES_PER_PAGE);
    let up_to_recursive_meta_pages_used = up_to_total_pages_used.div_ceil(ENTRIES_PER_PAGE - 5);
    let up_to_recursive_meta_phys_entries = up_to_recursive_meta_pages_used * 4;
    let up_to_recursive_meta_virt_entries = up_to_recursive_meta_pages_used * 1;

    let phys_entries_to_reserve = up_to_n_phys_entries + up_to_recursive_meta_phys_entries;
    let virt_entries_to_reserve = up_to_n_virt_entries + up_to_recursive_meta_virt_entries;

    // Continuous bool vec needs 1 extra
    let max_phys_vec_size = 1 + phys_entries_to_reserve * 2;
    // Continuous bool vec needs 1 extra
    let max_virt_vec_size = 1 + virt_entries_to_reserve * 2;

    let meta_data_size = size_of::<usize>() * (max_phys_vec_size + max_phys_vec_size);

    let meta_data_physical_start = memory
        .usable_physical_memory_regions()
        .find_map(|region| {
            let start = region.start.next_multiple_of(align_of::<usize>());
            let end = start + meta_data_size;
            if end <= region.end {
                Some(start)
            } else {
                None
            }
        })
        .unwrap();

    let mut temp_allocator_memory = memory.get_bytes_mut({
        let start = meta_data_physical_start + offset_map_start;
        start..start + meta_data_size
    });
    let temp_allocator = &BumpAllocator::from(temp_allocator_memory.deref_mut());
    #[cfg(test)]
    let temp_allocator = std::alloc::Global;

    // Note that we will not be allowed to realloc as dealloc is not implemented and realloc will panic
    let mut phys_mem_tracker = ContinuousBoolVec::new_2(
        true,
        usize::MAX,
        Vec::with_capacity_in(max_phys_vec_size, temp_allocator),
    );
    let mut kernel_address_space_tracker = ContinuousBoolVec::new_2(
        true,
        usize::MAX,
        Vec::with_capacity_in(max_virt_vec_size, temp_allocator),
    );

    // Mark the higher half as available
    kernel_address_space_tracker.set(0x800000000000..0x1000000000000, false);

    // Mark usable phys mem entries as available
    memory.usable_physical_memory_regions().for_each(|range| {
        phys_mem_tracker.set(range, false);
    });
    // Mark the phys mem that the meta data used as unavailable
    phys_mem_tracker.set(
        meta_data_physical_start..meta_data_physical_start + meta_data_size,
        true,
    );

    // Mark already mapped pages as unavailable virt areas
    PageTablesTraverser::new(
        memory,
        memory.get_current_top_level_page_table(),
        offset_map_start,
    )
    .for_each(|page_mapping| {
        let range = page_mapping.virt_start..page_mapping.virt_start + page_mapping.len as usize;
        kernel_address_space_tracker.set(range, true);
    });

    #[cfg(feature = "log")]
    log::info!(
        "Phys tracker: {:#?}. Virt tracker: {:#?}",
        phys_mem_tracker,
        kernel_address_space_tracker
    );

    // In a real environment, the initial trackers are basically backed by `static` memory which can't be reallocated.
    // We hackily switch the backing allocator to `StaticAllocator` to panic if it ever tries to reallocate.
    // Since we will have to add more items to the `Vec`, we will have to clone the static `Vec`s into a new one backed by the actual allocator.
    #[cfg(not(test))]
    let phys_mem_tracker = phys_mem_tracker.replace_len_vec(|len_vec| {
        let (a, b, c) = Vec::into_parts(len_vec);
        unsafe { Vec::from_parts_in(a, b, c, crate::static_allocator::StaticAllocator) }
    });
    #[cfg(not(test))]
    let kernel_address_space_tracker = kernel_address_space_tracker.replace_len_vec(|len_vec| {
        let (a, b, c) = Vec::into_parts(len_vec);
        unsafe { Vec::from_parts_in(a, b, c, crate::static_allocator::StaticAllocator) }
    });

    (phys_mem_tracker, kernel_address_space_tracker)
}

#[cfg(test)]
mod tests {
    use core::alloc::Layout;

    use crate::mock_memory::MockMemory;

    use super::*;

    #[test]
    pub fn test_scan() {
        let (memory, offset_map_start) =
            MockMemory::new_with_offset_map([0x0..0x10_000].into()).unwrap();
        // println!(
        //     "Memory: {:#?}. Offset map start: {}",
        //     memory, offset_map_start
        // );
        let (phys, virt) = scan_memory(&memory, offset_map_start);
        println!("Phys: {:#?}. Virt: {:#?}", phys, virt);
    }

    #[test]
    pub fn valid_single_byte() {
        let (memory, offset_map_start) =
            MockMemory::new_with_offset_map([0x0..0x10_000].into()).unwrap();
        let (phys_mem_tracker, kernel_address_space_tracker) =
            scan_memory(&memory, offset_map_start);

        #[derive(Debug)]
        struct NeededData {
            phys_mem_tracker: ContinuousBoolVec<Vec<usize>>,
            kernel_address_space_tracker: ContinuousBoolVec<Vec<usize>>,
        }
        impl GetTrackers for NeededData {
            fn get_phys_tracker(
                &mut self,
            ) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>> {
                &mut self.phys_mem_tracker
            }
            fn get_virt_tracker(
                &mut self,
            ) -> impl DerefMut<Target = ContinuousBoolVec<Vec<usize>>> {
                &mut self.kernel_address_space_tracker
            }
        }

        let needed_data = NeededData {
            phys_mem_tracker,
            kernel_address_space_tracker,
        };
        println!(
            "Data: {:#?}. Offset map start: {}",
            needed_data, offset_map_start
        );
        let mut allocator = PagingAllocator::new(needed_data, offset_map_start);
        let layout = Layout::from_size_align(1, 1).unwrap();
        let start = allocator.allocate(layout).unwrap().into();
        println!("ptr: {}", start);
        // Make sure memory is writable
        memory.get_bytes_mut(start..start + layout.size());
    }
}
