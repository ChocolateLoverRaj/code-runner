use core::ops::Range;

use x86_64::{
    registers::control::Cr3,
    structures::paging::{PageTable, PageTableFlags},
    PhysAddr, VirtAddr,
};

use crate::virt_addr_from_indexes::{virt_addr_from_indexes, virt_addr_from_indexes_4_kib};

pub struct PageMapping {
    pub virt_start: VirtAddr,
    pub phys_start: PhysAddr,
    pub len: u64,
}

pub fn traverse_cr3<F: FnMut(PageMapping)>(hhdm_offset: u64, mut callback: F) {
    let active_l4_pt = {
        let (active_l4, _cr3_flags) = Cr3::read();
        let active_l4_pt = (active_l4.start_address().as_u64() + hhdm_offset) as *const PageTable;
        unsafe { &*active_l4_pt }
    };

    let mut sub_pt_stack = heapless::Vec::<usize, 3>::new();
    let mut entry_index = 0;
    loop {
        // We went through every entry in the table
        if entry_index == 512 {
            if let Some(parent_entry_index) = sub_pt_stack.pop() {
                // We finished going through a L3, L2, or L1 page table, go to the next entry in the parent table
                entry_index = parent_entry_index + 1;
                continue;
            } else {
                // We finished going through the L4 page table so we're done
                break;
            }
        }
        // Get the current page table which has the entry that we're going to process
        let pt = {
            // Start with the L4 pt
            let mut pt = active_l4_pt;
            // Traverse the page tables until we get to the lowest level we want to process
            for index in &sub_pt_stack {
                let pt_ptr = (pt[*index].addr().as_u64() + hhdm_offset) as *const PageTable;
                pt = unsafe { &*pt_ptr };
            }
            pt
        };
        let entry = &pt[entry_index];
        if !entry.is_unused() && entry.flags().contains(PageTableFlags::PRESENT) {
            if entry.flags().contains(PageTableFlags::HUGE_PAGE) || sub_pt_stack.len() == 3 {
                // This entry point to a phys frame, which could be 4KiB, 2MiB, or 1GiB
                // Note that just cuz PageTableFlags::HUGE_PAGE is 1 doesn't mean that it's >4KiB - see https://github.com/phil-opp/blog_os/issues/1403
                log::debug!(
                    "L{} entry - phys frame at {:?}[{}]",
                    4 - sub_pt_stack.len(),
                    sub_pt_stack,
                    entry_index
                );
                callback(PageMapping {
                    virt_start: virt_addr_from_indexes(
                        &{
                            let mut indexes =
                                heapless::Vec::<_, 4>::from_slice(&sub_pt_stack).unwrap();
                            indexes.push(entry_index).unwrap();
                            indexes
                        },
                        0,
                    ),
                    phys_start: entry.addr(),
                    len: 0x1000 * 512_u64.pow((3 - sub_pt_stack.len()).try_into().unwrap()),
                });
                entry_index += 1;
            } else {
                // This entry points to another entry
                sub_pt_stack.push(entry_index).unwrap();
                entry_index = 0;
                continue;
            }
        } else {
            entry_index += 1;
        }
    }
}

pub fn traverse_cr3_and_print_size(hhdm_offset: u64) {
    let active_l4_pt = {
        let (active_l4, _cr3_flags) = Cr3::read();
        let active_l4_pt = (active_l4.start_address().as_u64() + hhdm_offset) as *const PageTable;
        unsafe { &*active_l4_pt }
    };

    let mut mapped_bytes = 0;
    let mut sub_pt_stack = heapless::Vec::<usize, 3>::new();
    let mut entry_index = 0;
    loop {
        // We went through every entry in the table
        if entry_index == 512 {
            if let Some(parent_entry_index) = sub_pt_stack.pop() {
                // We finished going through a L3, L2, or L1 page table, go to the next entry in the parent table
                entry_index = parent_entry_index + 1;
                continue;
            } else {
                // We finished going through the L4 page table so we're done
                break;
            }
        }
        // Get the current page table which has the entry that we're going to process
        let pt = {
            // Start with the L4 pt
            let mut pt = active_l4_pt;
            // Traverse the page tables until we get to the lowest level we want to process
            for index in &sub_pt_stack {
                let pt_ptr = (pt[*index].addr().as_u64() + hhdm_offset) as *const PageTable;
                pt = unsafe { &*pt_ptr };
            }
            pt
        };
        let entry = &pt[entry_index];
        if !entry.is_unused() && entry.flags().contains(PageTableFlags::PRESENT) {
            if entry.flags().contains(PageTableFlags::HUGE_PAGE) || sub_pt_stack.len() == 3 {
                // This entry point to a phys frame, which could be 4KiB, 2MiB, or 1GiB
                // Note that just cuz PageTableFlags::HUGE_PAGE is 1 doesn't mean that it's >4KiB - see https://github.com/phil-opp/blog_os/issues/1403
                log::debug!(
                    "L{} entry - phys frame at {:?}[{}]",
                    4 - sub_pt_stack.len(),
                    sub_pt_stack,
                    entry_index
                );
                mapped_bytes +=
                    0x1000 * 512_usize.pow((3 - sub_pt_stack.len()).try_into().unwrap());
                entry_index += 1;
            } else {
                // This entry points to another entry
                sub_pt_stack.push(entry_index).unwrap();
                entry_index = 0;
                continue;
            }
        } else {
            entry_index += 1;
        }
    }

    log::info!(
        "The current Cr3 has a total of 0x{:X} bytes mapped",
        mapped_bytes
    );
}
