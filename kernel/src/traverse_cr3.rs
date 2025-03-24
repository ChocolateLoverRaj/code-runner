use x86_64::{
    registers::control::Cr3,
    structures::paging::{PageTable, PageTableFlags},
    PhysAddr, VirtAddr,
};

use crate::virt_addr_from_indexes::virt_addr_from_indexes;

pub struct PageMapping {
    pub virt_start: VirtAddr,
    pub phys_start: PhysAddr,
    pub len: u64,
}

/// Recursively traverses page table, returning every mapping
pub struct PageTableDeepIterator {
    hhdm_offset: u64,
    sub_pt_stack: heapless::Vec<usize, 3>,
    entry_index: usize,
}

impl PageTableDeepIterator {
    /// # Safety
    /// Memory must actually mapped according to the Limine protocol's HHDM
    pub unsafe fn new(hhdm_offset: u64) -> Self {
        Self {
            hhdm_offset,
            sub_pt_stack: Default::default(),
            entry_index: Default::default(),
        }
    }
}

impl Iterator for PageTableDeepIterator {
    type Item = PageMapping;

    fn next(&mut self) -> Option<Self::Item> {
        let active_l4_pt = {
            let (active_l4, _cr3_flags) = Cr3::read();
            let active_l4_pt =
                (active_l4.start_address().as_u64() + self.hhdm_offset) as *const PageTable;
            unsafe { &*active_l4_pt }
        };

        loop {
            // We went through every entry in the table
            if self.entry_index == 512 {
                if let Some(parent_entry_index) = self.sub_pt_stack.pop() {
                    // We finished going through a L3, L2, or L1 page table, go to the next entry in the parent table
                    self.entry_index = parent_entry_index + 1;
                    continue;
                } else {
                    // We finished going through the L4 page table so we're done
                    break None;
                }
            }
            // Get the current page table which has the entry that we're going to process
            let pt = {
                // Start with the L4 pt
                let mut pt = active_l4_pt;
                // Traverse the page tables until we get to the lowest level we want to process
                for index in &self.sub_pt_stack {
                    let pt_ptr =
                        (pt[*index].addr().as_u64() + self.hhdm_offset) as *const PageTable;
                    pt = unsafe { &*pt_ptr };
                }
                pt
            };
            let entry = &pt[self.entry_index];
            if !entry.is_unused() && entry.flags().contains(PageTableFlags::PRESENT) {
                if entry.flags().contains(PageTableFlags::HUGE_PAGE) || self.sub_pt_stack.len() == 3
                {
                    // This entry point to a phys frame, which could be 4KiB, 2MiB, or 1GiB
                    // Note that just cuz PageTableFlags::HUGE_PAGE is 1 doesn't mean that it's >4KiB - see https://github.com/phil-opp/blog_os/issues/1403
                    let page_mapping = PageMapping {
                        virt_start: virt_addr_from_indexes(
                            &{
                                let mut indexes =
                                    heapless::Vec::<_, 4>::from_slice(&self.sub_pt_stack).unwrap();
                                indexes.push(self.entry_index).unwrap();
                                indexes
                            },
                            0,
                        ),
                        phys_start: entry.addr(),
                        len: 0x1000
                            * 512_u64.pow((3 - self.sub_pt_stack.len()).try_into().unwrap()),
                    };
                    self.entry_index += 1;
                    break Some(page_mapping);
                } else {
                    // This entry points to another entry
                    self.sub_pt_stack.push(self.entry_index).unwrap();
                    self.entry_index = 0;
                    continue;
                }
            } else {
                self.entry_index += 1;
            }
        }
    }
}
