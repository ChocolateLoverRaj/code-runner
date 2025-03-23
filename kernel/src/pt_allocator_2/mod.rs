pub mod initial_meta_allocator;
pub mod temp_frame_allocator;

use core::{
    alloc::GlobalAlloc,
    cell::RefCell,
    num::NonZeroUsize,
    ops::{DerefMut, Range},
    ptr::NonNull,
};

use alloc::{
    alloc::{AllocError, Allocator},
    vec::Vec,
};
use conquer_once::noblock::OnceCell;
use initial_meta_allocator::InitialMetaAllocator;
use limine::{memory_map::EntryType, response::MemoryMapResponse};
use spinning_top::Spinlock;
use temp_frame_allocator::TempFrameAllocator;
use util::{continuous_bool_vec::ContinuousBoolVec, continuous_bool_vec_2::ContinuousBoolVec2};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::{
    not_const_allocator::NotConstAllocator,
    traverse_cr3::{traverse_cr3, PageTableDeepIterator},
    virt_addr_to_number::VirtAddrToNumber,
};

pub const PHYS_MEM_TRACKER: OnceCell<Spinlock<ContinuousBoolVec2<Vec<u8>>>> = OnceCell::uninit();
pub const KERNEL_ADDRESS_SPACE_TRACKER: OnceCell<Spinlock<ContinuousBoolVec2<Vec<u8>>>> =
    OnceCell::uninit();

pub struct PtAllocator {
    memory_map_response: &'static MemoryMapResponse,
    hhdm_offset: u64,
    // phys_mem_tracker: &'static Spinlock<ContinuousBoolVec2<Vec<u8>>>,
    // kernel_address_space_tracker: &'static Spinlock<ContinuousBoolVec2<Vec<u8>>>,
}

impl PtAllocator {
    pub fn new(memory_map_response: &'static MemoryMapResponse, hhdm_offset: u64) -> Self {
        let used_phys_bytes = Default::default();
        let just_used_phys_frames = Default::default();
        let just_used_pages = Default::default();
        // We need N to be 8 because making the phys_mem_tracker could use 4 items and making the kernel_address_space_tracker could use 4 items
        let initial_meta_allocator = InitialMetaAllocator::<8> {
            hhdm_offset,
            used_phys_bytes: &used_phys_bytes,
            memory_map_response,
            just_used_phys_frames: &just_used_phys_frames,
            just_used_pages: &just_used_pages,
        };

        // let phys_start = memory_map_response
        //     .entries()
        //     .iter()
        //     .find(|entry| {
        //         entry.entry_type == EntryType::USABLE
        //             && entry.length as usize >= initial_tracker_size
        //     })
        //     .unwrap();

        let mut phys_mem_tracker = ContinuousBoolVec::new_2(
            true,
            usize::MAX,
            Vec::with_capacity_in(0x1000 / size_of::<usize>(), initial_meta_allocator.clone()),
        );
        let mut kernel_address_space_tracker = ContinuousBoolVec::new_2(
            true,
            usize::MAX,
            Vec::with_capacity_in(0x1000 / size_of::<usize>(), initial_meta_allocator.clone()),
        );

        // So we have the following regions
        // Initially, all mem is marked as unavailable (true)
        // We have some mem that should be marked as available (false)
        // We want to mark the memory used by the initial_meta_allocator as unavailable (true)

        // We will do this by initially marking the memory used by the initial_meta_allocator as `false`
        // Then we will invert all memory that Limine said is available

        while let Some(phys_frame) = just_used_phys_frames.borrow_mut().pop() {
            let start = phys_frame.start_address().as_u64() as usize;
            phys_mem_tracker.set(start..start + phys_frame.size() as usize, false);
        }

        while let Some(page) = just_used_pages.borrow_mut().pop() {
            let start = page.start_address().as_u64() as usize;
            kernel_address_space_tracker.set(start..start + page.size() as usize, false);
        }

        log::info!(
            "Phys mem tracker: {:?}; Kernel virt tracker: {:?} {:?} {:?}",
            phys_mem_tracker,
            kernel_address_space_tracker,
            just_used_phys_frames,
            just_used_pages
        );

        memory_map_response
            .entries()
            .iter()
            .filter(|entry| entry.entry_type == EntryType::USABLE)
            .for_each(|entry| {
                let start = entry.base as usize;
                let range = start..start + entry.length as usize;
                log::info!("Range: {:X?}", range);
            });

        todo!();

        Self {
            memory_map_response,
            hhdm_offset,
        }
    }
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
    ALLOCATOR
        .init(PtAllocator::new(memory_map_response, hhdm_offset))
        .unwrap();
}
