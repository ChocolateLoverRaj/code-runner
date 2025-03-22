use core::{alloc::GlobalAlloc, ops::Range};

use limine::{memory_map::EntryType, response::MemoryMapResponse};
use spinning_top::Spinlock;
use util::continuous_bool_vec::ContinuousBoolVec;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::{not_const_allocator::NotConstAllocator, traverse_cr3::traverse_cr3};

struct PtAllocatorState {
    /// Used for keeping track of how many phys frames we used
    // FIXME: Have a way of dynamically growing this vec
    phys_mem: ContinuousBoolVec<heapless::Vec<usize, 200>>,
    virt_mem: ContinuousBoolVec<heapless::Vec<usize, 200>>,
}

pub struct PtAllocator {
    memory_map_response: &'static MemoryMapResponse,
    hhdm_offset: u64,
    state: Spinlock<PtAllocatorState>,
}

impl PtAllocator {
    pub fn new(memory_map_response: &'static MemoryMapResponse, hhdm_offset: u64) -> Self {
        Self {
            state: Spinlock::new(PtAllocatorState {
                phys_mem: {
                    let mut v = ContinuousBoolVec::new(usize::MAX, true);
                    memory_map_response
                        .entries()
                        .iter()
                        .filter(|entry| entry.entry_type == EntryType::USABLE)
                        .for_each(|entry| {
                            v.set(
                                entry.base as usize..(entry.base + entry.length) as usize,
                                false,
                            )
                        });
                    v
                },
                virt_mem: {
                    // Mark all virt memory as unavailable
                    let mut v = ContinuousBoolVec::new(usize::MAX, true);
                    // Mark upper half as available
                    v.set(0x800000000000..0xFFFFFFFFFFFF, false);
                    // Mark all used mem as unavailable
                    traverse_cr3(hhdm_offset, |page_mapping| {
                        let start =
                            (page_mapping.virt_start.as_u64() & 0x0000_FFFF_FFFF_FFFF) as usize;
                        let range = start..start + page_mapping.len as usize;
                        log::debug!("Used virt range: {:X?}", range);
                        v.set(range, true);
                    });
                    v
                },
            }),
            memory_map_response,
            hhdm_offset,
        }
    }
}

unsafe impl GlobalAlloc for PtAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut state = self.state.lock();

        let phys_start = state
            .phys_mem
            .get_continuous_range_with_alignment(false, layout.size(), layout.align())
            .expect("Could not alloc - ran out of physical memory");
        state
            .phys_mem
            .set(phys_start..phys_start + layout.size(), true);

        let virt_len = layout.size().div_ceil(Size4KiB::SIZE as usize) * Size4KiB::SIZE as usize;
        let virt_page_start = state
            .virt_mem
            .get_continuous_range_with_alignment(
                false,
                virt_len,
                layout.align().max(Size4KiB::SIZE as usize),
            )
            .unwrap();
        state
            .virt_mem
            .set(virt_page_start..virt_page_start + virt_len, true);

        let virt_actual_start = virt_page_start + (phys_start % Size4KiB::SIZE as usize);

        let (active_l4, _cr3_flags) = Cr3::read();
        let active_l4_pt =
            (active_l4.start_address().as_u64() + self.hhdm_offset) as *mut PageTable;
        let active_l4_pt = unsafe { &mut *active_l4_pt };
        let mut offset_page_table =
            unsafe { OffsetPageTable::new(active_l4_pt, VirtAddr::new(self.hhdm_offset)) };

        struct MyFrameAllocator<'a> {
            state: &'a mut PtAllocatorState,
        }
        unsafe impl FrameAllocator<Size4KiB> for MyFrameAllocator<'_> {
            fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
                let start = self.state.phys_mem.get_continuous_range_with_alignment(
                    false,
                    Size4KiB::SIZE as usize,
                    Size4KiB::SIZE as usize,
                )?;
                self.state
                    .phys_mem
                    .set(start..start + Size4KiB::SIZE as usize, true);
                Some(PhysFrame::from_start_address(PhysAddr::new(start as u64)).unwrap())
            }
        }

        let page_count = align_range(
            virt_actual_start..virt_actual_start + layout.size(),
            Size4KiB::SIZE as usize,
        )
        .len()
            / Size4KiB::SIZE as usize;
        for i in 0..page_count {
            let page = Page::<Size4KiB>::from_start_address(VirtAddr::new_truncate(
                virt_page_start as u64,
            ))
            .unwrap()
                + i as u64;
            log::debug!(
                "Mapping page starting at 0x{:X}",
                page.start_address().as_u64() & 0x0000_FFFF_FFFF_FFFF,
            );
            unsafe {
                offset_page_table
                    .map_to(
                        page,
                        PhysFrame::containing_address(PhysAddr::new(phys_start as u64)) + i as u64,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::NO_EXECUTE,
                        &mut MyFrameAllocator { state: &mut state },
                    )
                    .unwrap()
                    // TODO: Use PCID / at least avoid unnecessary flushing
                    .flush();
            }
        }

        VirtAddr::new_truncate(virt_actual_start as u64).as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        log::warn!("Leaking memory cuz dealloc not implemented yet");
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
