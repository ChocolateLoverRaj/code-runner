use common::mem::KERNEL_VIRT_MEM_START;
use core::ops::Range;
use x86_64::{
    structures::paging::{
        page_table::FrameError, PageOffset, PageSize, PageTable, PageTableIndex, Size1GiB,
        Size2MiB, Size4KiB,
    },
    VirtAddr,
};

use crate::{
    virt_addr_from_indexes::{
        virt_addr_from_indexes_1_gib, virt_addr_from_indexes_2_mib, virt_addr_from_indexes_4_kib,
    },
    virt_mem_tracker::VirtMemTracker,
};

pub fn find_used_virt_addrs(
    l4_page_table: &PageTable,
    phys_mem_offset: VirtAddr,
    virt_mem_tracker: &mut VirtMemTracker,
) {
    // We assume that this function is being called in an increasing way (0..2, 2..4, 10..16), not (10..16, 1..2)
    let mut add_range = |range: Range<VirtAddr>| {
        if range.start >= VirtAddr::new(KERNEL_VIRT_MEM_START) {
            virt_mem_tracker.allocate_specific_bytes_unchecked(range);
        }
    };
    for (l4_index, entry) in l4_page_table.iter().enumerate() {
        match entry.frame() {
            Ok(frame) => {
                let virt_frame_addr = phys_mem_offset + frame.start_address().as_u64();
                let l3_table_ptr: *const PageTable = virt_frame_addr.as_ptr();
                let l3_table = unsafe { &*l3_table_ptr };
                for (l3_index, entry) in l3_table.iter().enumerate() {
                    match entry.frame() {
                        Ok(frame) => {
                            let virt_frame_addr = phys_mem_offset + frame.start_address().as_u64();
                            let l2_table_ptr: *const PageTable = virt_frame_addr.as_ptr();
                            let l2_table = unsafe { &*l2_table_ptr };
                            for (l2_index, entry) in l2_table.iter().enumerate() {
                                match entry.frame() {
                                    Ok(frame) => {
                                        let virt_frame_addr =
                                            phys_mem_offset + frame.start_address().as_u64();
                                        let l1_table_ptr: *const PageTable =
                                            virt_frame_addr.as_ptr();
                                        let l1_table = unsafe { &*l1_table_ptr };
                                        for (l1_index, entry) in l1_table.iter().enumerate() {
                                            if !entry.is_unused() {
                                                add_range({
                                                    let start_addr = virt_addr_from_indexes_4_kib(
                                                        PageTableIndex::new(l4_index as u16),
                                                        PageTableIndex::new(l3_index as u16),
                                                        PageTableIndex::new(l2_index as u16),
                                                        PageTableIndex::new(l1_index as u16),
                                                        PageOffset::new(0),
                                                    );
                                                    Range {
                                                        start: start_addr,
                                                        end: start_addr + Size4KiB::SIZE,
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    Err(FrameError::FrameNotPresent) => {}
                                    Err(FrameError::HugeFrame) => {
                                        add_range({
                                            let start_addr = virt_addr_from_indexes_2_mib(
                                                PageTableIndex::new(l4_index as u16),
                                                PageTableIndex::new(l3_index as u16),
                                                PageTableIndex::new(l2_index as u16),
                                                0,
                                            );
                                            Range {
                                                start: start_addr,
                                                end: start_addr + Size2MiB::SIZE,
                                            }
                                        });
                                    }
                                }
                            }
                        }
                        Err(FrameError::FrameNotPresent) => {}
                        Err(FrameError::HugeFrame) => add_range({
                            let start_addr = virt_addr_from_indexes_1_gib(
                                PageTableIndex::new(l4_index as u16),
                                PageTableIndex::new(l3_index as u16),
                                0,
                            );
                            Range {
                                start: start_addr,
                                end: start_addr + Size1GiB::SIZE,
                            }
                        }),
                    }
                }
            }
            Err(FrameError::FrameNotPresent) => {}
            Err(FrameError::HugeFrame) => {
                panic!("An L4 page table entry was a huge. This is not allowed.")
            }
        }
    }
}
