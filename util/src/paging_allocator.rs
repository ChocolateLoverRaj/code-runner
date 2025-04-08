use alloc::vec::Vec;
use core::{
    alloc::AllocError,
    ops::{Deref, DerefMut},
};

use crate::{
    allocator_test::{Memory, PageTable, TestableAllocator},
    bump_allocator::BumpAllocator,
    continuous_bool_vec::ContinuousBoolVec,
    traverse_page_tables::PageTablesTraverser,
};

struct PagingAllocator {
    // phys_mem: ContinuousBoolVec,
}

impl TestableAllocator for PagingAllocator {
    fn allocate(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<u8>, AllocError> {
        Err(AllocError)
    }
}

impl PagingAllocator {
    pub fn init<'a, T: Memory>(memory: &'a T, offset_map_start: usize) -> Self {
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
        let temp_allocator = BumpAllocator::from(temp_allocator_memory.deref_mut());

        // Note that we will not be allowed to realloc as dealloc is not implemented and realloc will panic
        let mut phys_mem_tracker = ContinuousBoolVec::new_2(
            true,
            usize::MAX,
            Vec::with_capacity_in(max_phys_vec_size, &temp_allocator),
        );
        let mut kernel_address_space_tracker = ContinuousBoolVec::new_2(
            true,
            usize::MAX,
            Vec::with_capacity_in(max_virt_vec_size, &temp_allocator),
        );

        Self {}
    }
}

#[cfg(test)]
mod tests {
    use crate::mock_memory::MockMemory;

    use super::*;

    #[test]
    pub fn test_init() {
        let (memory, offset_map_start) =
            MockMemory::new_with_offset_map([0x0..0x10_000].into()).unwrap();
        // println!(
        //     "Memory: {:#?}. Offset map start: {}",
        //     memory, offset_map_start
        // );
        PagingAllocator::init(&memory, offset_map_start);
    }
}
