pub mod initial_meta_allocator;
pub mod temp_frame_allocator;

use core::{alloc::GlobalAlloc, ops::Range};

use alloc::vec::Vec;
use initial_meta_allocator::InitialMetaAllocator;
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use spinning_top::Spinlock;
use util::{continuous_bool_vec::ContinuousBoolVec, init_later::InitLater};

use crate::{
    not_const_allocator::NotConstAllocator, traverse_cr3::PageTableDeepIterator,
    virt_addr_to_number::VirtAddrToNumber,
};

pub static PHYS_MEM_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();
pub static PHYS_MEM_USED_BY_KERNEL: InitLater<Spinlock<usize>> = InitLater::uninit();
pub static KERNEL_ADDRESS_SPACE_TRACKER: InitLater<Spinlock<ContinuousBoolVec<Vec<usize>>>> =
    InitLater::uninit();

pub struct PtAllocator {
    memory_map_response: &'static MemoryMapResponse,
    hhdm_offset: u64,
}

unsafe impl GlobalAlloc for PtAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }

    // We don't implement alloc_zeroed cuz we don't have a faster way of doing this that's faster than what the default impl would do
    // We might be able to increase performance in the future by zeroing some phys frame in advance and implementing alloc_zeroed ourselves

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        todo!()
    }
}

fn align_range(range: Range<usize>, alignment: usize) -> Range<usize> {
    range.start.div_floor(alignment) * alignment..range.end.div_ceil(alignment) * alignment
}

#[global_allocator]
static ALLOCATOR: NotConstAllocator<PtAllocator> = NotConstAllocator::uninit();

pub fn init(memory_map_response: &'static MemoryMapResponse, hhdm_offset: u64) {
    let used_phys_bytes = Default::default();
    // We need N to be 8 because making the phys_mem_tracker could use 4 items and making the kernel_address_space_tracker could use 4 items
    let initial_meta_allocator = InitialMetaAllocator {
        hhdm_offset,
        used_phys_bytes: &used_phys_bytes,
        memory_map_response,
    };

    // let phys_start = memory_map_response
    //     .entries()
    //     .iter()
    //     .find(|entry| {
    //         entry.entry_type == EntryType::USABLE
    //             && entry.length as usize >= initial_tracker_size
    //     })
    //     .unwrap();

    // Calculate the number of pages. These pages could be >4KiB, and that's okay
    const PAGE_SIZE: usize = 0x1000;
    const ENTRY_SIZE: usize = size_of::<u64>() * 2;

    // Calculate the max vec size needed (for both phys and virt)
    // For every 256 entries (each entry up to 2 x u64, so 16B), we will need 1 page
    // For every page, there will be up to 4 phys frames and 1 page used
    let up_to_n_phys_entries_after_allocating = {
        let up_to_n_phys_entries =
            1 + unsafe { PageTableDeepIterator::new(hhdm_offset) }.count() * 4;
        const PHYS_ENTRIES_PER_PAGE: usize = PAGE_SIZE.div_floor(ENTRY_SIZE * 4);
        (up_to_n_phys_entries * PHYS_ENTRIES_PER_PAGE).div_ceil(PHYS_ENTRIES_PER_PAGE - 1)
    };

    let up_to_n_virt_entries_after_allocating = {
        let up_to_n_virt_entries = 1 + unsafe { PageTableDeepIterator::new(hhdm_offset) }.count();
        const VIRT_ENTRIES_PER_PAGE: usize = PAGE_SIZE.div_floor(ENTRY_SIZE);
        (up_to_n_virt_entries * VIRT_ENTRIES_PER_PAGE).div_ceil(VIRT_ENTRIES_PER_PAGE - 1)
    };

    let max_phys_vec_size = 1 + up_to_n_phys_entries_after_allocating * 2;
    let max_virt_vec_size = 2 + up_to_n_virt_entries_after_allocating * 2;

    // Note that we will not be allowed to realloc as dealloc is not implemented and realloc will panic
    let mut phys_mem_tracker = ContinuousBoolVec::new_2(
        true,
        usize::MAX,
        Vec::with_capacity_in(max_phys_vec_size, initial_meta_allocator.clone()),
    );
    let mut kernel_address_space_tracker = ContinuousBoolVec::new_2(
        true,
        usize::MAX,
        Vec::with_capacity_in(max_virt_vec_size, initial_meta_allocator.clone()),
    );
    kernel_address_space_tracker.set(0x800000000000..0x1000000000000, false);

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
            phys_mem_tracker.set(start..start + subtract_amount, true);
            used_available_phy_mem -= subtract_amount;
        } else {
            break;
        }
    }

    memory_map_response
        .entries()
        .iter()
        .filter(|entry| entry.entry_type == EntryType::USABLE)
        .for_each(|entry| {
            let start = entry.base as usize;
            let range = start..start + entry.length as usize;
            phys_mem_tracker.set(range, false);
        });
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

    PHYS_MEM_USED_BY_KERNEL
        .try_init(Spinlock::new(used_phys_bytes.take()))
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
        .try_init(PtAllocator {
            memory_map_response,
            hhdm_offset,
        })
        .unwrap();
}
