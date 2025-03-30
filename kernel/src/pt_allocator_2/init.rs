use alloc::vec::Vec;
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use spinning_top::Spinlock;
use util::continuous_bool_vec::ContinuousBoolVec;

use crate::{
    hhdm_offset::HhdmOffset,
    pt_allocator_2::{
        initial_meta_allocator::InitialMetaAllocator, pt_allocator_2::PtAllocator2, ALLOCATOR,
        KERNEL_ADDRESS_SPACE_TRACKER, MEMORY_USAGE_STATS, PHYS_MEM_TRACKER,
    },
    traverse_cr3::PageTableDeepIterator,
    virt_addr_to_number::VirtAddrToNumber,
};

pub fn init(memory_map_response: &'static MemoryMapResponse, hhdm_offset: HhdmOffset) {
    let used_phys_bytes = Default::default();
    // We need N to be 8 because making the phys_mem_tracker could use 4 items and making the kernel_address_space_tracker could use 4 items
    let memory_usage_stats = Default::default();
    let initial_meta_allocator = InitialMetaAllocator {
        hhdm_offset,
        used_phys_bytes: &used_phys_bytes,
        memory_map_response,
        memory_usage_stats: &memory_usage_stats,
    };

    // Calculate the number of pages. These pages could be >4KiB, and that's okay
    const PAGE_SIZE: usize = 0x1000;
    const ENTRY_SIZE: usize = size_of::<u64>() * 2;
    const ENTRIES_PER_PAGE: usize = PAGE_SIZE / ENTRY_SIZE;

    // Calculate the max vec size needed (for both phys and virt)
    // For every 256 entries (each entry up to 2 x u64, so 16B), we will need 1 page
    // For every page, there will be up to 4 phys frames and 1 page used
    let up_to_n_phys_entries = memory_map_response
        .entries()
        .iter()
        .filter(|entry| entry.entry_type == EntryType::USABLE)
        .count();
    let up_to_n_virt_entries = {
        let mut disconnected_ranges = 0_usize;
        let mut end = None;
        unsafe { PageTableDeepIterator::new(hhdm_offset) }.for_each(|mapping| {
            if end != Some(mapping.virt_start) {
                disconnected_ranges += 1;
                end = Some(mapping.virt_start + mapping.len);
            } else {
                end = Some(mapping.virt_start + mapping.len);
            }
        });
        disconnected_ranges
    };

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
    log::info!(
        "Up to entries: phys: {}. virt: {}. Reserving phys entries: {}. Reserving virt entries: {}",
        up_to_n_phys_entries,
        up_to_n_virt_entries,
        phys_entries_to_reserve,
        virt_entries_to_reserve
    );

    // Reserve a multiple of a page so that actually used memory matches with the reserved memory
    const U64_PER_PAGE: usize = PAGE_SIZE / size_of::<u64>();

    // Note that we will not be allowed to realloc as dealloc is not implemented and realloc will panic
    let mut phys_mem_tracker = ContinuousBoolVec::new_2(
        true,
        usize::MAX,
        Vec::with_capacity_in(
            max_phys_vec_size.next_multiple_of(U64_PER_PAGE),
            initial_meta_allocator.clone(),
        ),
    );
    let mut kernel_address_space_tracker = ContinuousBoolVec::new_2(
        true,
        usize::MAX,
        Vec::with_capacity_in(
            max_virt_vec_size.next_multiple_of(U64_PER_PAGE),
            initial_meta_allocator.clone(),
        ),
    );
    // Mark the higher half as available
    kernel_address_space_tracker.set(0x800000000000..0x1000000000000, false);

    // Mark usable phys mem entries as available
    memory_map_response
        .entries()
        .iter()
        .filter(|entry| entry.entry_type == EntryType::USABLE)
        .for_each(|entry| {
            let start = entry.base as usize;
            let range = start..start + entry.length as usize;
            phys_mem_tracker.set(range, false);
        });
    // Mark the phys mem that the temp allocator used as unavailable
    let mut used_available_phy_mem = *used_phys_bytes.borrow();
    let mut iter = memory_map_response
        .entries()
        .iter()
        .filter(|entry| entry.entry_type == EntryType::USABLE);
    loop {
        if used_available_phy_mem == 0 {
            break;
        }
        if let Some(entry) = iter.next() {
            let subtract_amount = (entry.length as usize).min(used_available_phy_mem);
            let start = entry.base as usize;
            let range = start..start + subtract_amount;
            log::debug!("Phys range used by allocator metadata: {:X?}", range);
            phys_mem_tracker.set(range, true);
            used_available_phy_mem -= subtract_amount;
        } else {
            break;
        }
    }

    // Mark already mapped pages as unavailable virt areas
    unsafe { PageTableDeepIterator::new(hhdm_offset) }.for_each(|page_mapping| {
        let start = page_mapping.virt_start.into_number();
        let range = start..start + page_mapping.len as usize;
        kernel_address_space_tracker.set(range, true);
    });
    log::debug!(
        "Phys mem tracker: {:#?}; Kernel virt tracker: {:#?}",
        phys_mem_tracker,
        kernel_address_space_tracker,
    );

    // Convert the vec to be backed by the global allocator instead of the temp allocator
    PHYS_MEM_TRACKER
        .try_init({
            Spinlock::new(ContinuousBoolVec {
                start_value: phys_mem_tracker.start_value,
                len_vec: {
                    let (ptr, len, cap) = phys_mem_tracker.len_vec.into_parts();
                    unsafe { Vec::from_parts(ptr, len, cap) }
                },
            })
        })
        .unwrap();

    MEMORY_USAGE_STATS
        .try_init(Spinlock::new(memory_usage_stats.take()))
        .unwrap();

    KERNEL_ADDRESS_SPACE_TRACKER
        .try_init({
            Spinlock::new(ContinuousBoolVec {
                start_value: kernel_address_space_tracker.start_value,
                len_vec: {
                    let (ptr, len, cap) = kernel_address_space_tracker.len_vec.into_parts();
                    unsafe { Vec::from_parts(ptr, len, cap) }
                },
            })
        })
        .unwrap();

    ALLOCATOR
        .try_init(PtAllocator2 {
            memory_map_response,
            hhdm_offset,
        })
        .unwrap();
}
