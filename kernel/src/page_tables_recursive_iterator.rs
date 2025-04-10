use x86_64::{
    structures::paging::{PageTable, PageTableFlags, PhysFrame, Size4KiB},
    VirtAddr,
};

use crate::{hhdm_offset::HhdmOffset, virt_addr_from_indexes::virt_addr_from_indexes};

#[derive(Debug, Clone)]
pub struct PageTableIndexStack {
    stack: heapless::Vec<usize, 4>,
}

impl PageTableIndexStack {
    pub fn start_address(&self) -> VirtAddr {
        virt_addr_from_indexes(&self.stack, 0)
    }

    /// Get the length of the page table (or missing entry) as a multiple of 4KiB
    pub fn n_4kib_pages(&self) -> u64 {
        512_u64.pow((4 - self.stack.len()).try_into().unwrap())
    }

    /// Get the length of the page table (or missing entry)
    pub fn page_len(&self) -> u64 {
        0x1000 * self.n_4kib_pages()
    }
}

#[derive(Debug, Clone)]
pub struct PageTableEntry {
    pub page_table_index_stack: PageTableIndexStack,
    /// If the present bit is set in the flags
    pub present: bool,
}

/// Recursively traverses page table, returning every mapping
pub struct PageTablesRecursiveIterator {
    hhdm_offset: HhdmOffset,
    top_level_page_table: PhysFrame<Size4KiB>,
    parent_page_tables_entry_index_stack: heapless::Vec<usize, 3>,
    entry_index: usize,
}

impl PageTablesRecursiveIterator {
    /// The `initial_entry_index` is initial entry index in the top level page table to start at.
    /// For example, use 256 to start in the higher half of the virtual address space.
    ///
    /// # Safety
    /// The top level page table and its entries must be actually mapped
    pub unsafe fn new(
        hhdm_offset: HhdmOffset,
        top_level_page_table: PhysFrame<Size4KiB>,
        initial_entry_index: usize,
    ) -> Self {
        Self {
            hhdm_offset,
            top_level_page_table,
            parent_page_tables_entry_index_stack: Default::default(),
            entry_index: initial_entry_index,
        }
    }
}

impl Iterator for PageTablesRecursiveIterator {
    type Item = PageTableEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let active_l4_pt = {
            let active_l4_pt = (self.top_level_page_table.start_address().as_u64()
                + u64::from(self.hhdm_offset)) as *const PageTable;
            unsafe { &*active_l4_pt }
        };

        loop {
            // We went through every entry in the table
            if self.entry_index == 512 {
                if let Some(parent_entry_index) = self.parent_page_tables_entry_index_stack.pop() {
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
                for index in &self.parent_page_tables_entry_index_stack {
                    let pt_ptr = (pt[*index].addr().as_u64() + u64::from(self.hhdm_offset))
                        as *const PageTable;
                    pt = unsafe { &*pt_ptr };
                }
                pt
            };
            let entry = &pt[self.entry_index];
            let get_page_table_index_stack = || {
                // let start = virt_addr_from_indexes(
                //     &{
                //         let mut indexes = heapless::Vec::<_, 4>::from_slice(
                //             &self.parent_page_tables_entry_index_stack,
                //         )
                //         .unwrap();
                //         indexes.push(self.entry_index).unwrap();
                //         indexes
                //     },
                //     0,
                // );
                // let len = 0x1000
                //     * 512_u64.pow(
                //         (3 - self.parent_page_tables_entry_index_stack.len())
                //             .try_into()
                //             .unwrap(),
                //     );
                PageTableIndexStack {
                    stack: {
                        let mut indexes = heapless::Vec::<_, 4>::from_slice(
                            &self.parent_page_tables_entry_index_stack,
                        )
                        .unwrap();
                        indexes.push(self.entry_index).unwrap();
                        indexes
                    },
                }
            };
            if entry.flags().contains(PageTableFlags::PRESENT) {
                if entry.flags().contains(PageTableFlags::HUGE_PAGE)
                    || self.parent_page_tables_entry_index_stack.len() == 3
                {
                    // This entry point to a phys frame, which could be 4KiB, 2MiB, or 1GiB
                    // Note that just cuz PageTableFlags::HUGE_PAGE is 1 doesn't mean that it's >4KiB - see https://github.com/phil-opp/blog_os/issues/1403
                    let page_table_entry = PageTableEntry {
                        page_table_index_stack: get_page_table_index_stack(),
                        present: true,
                    };
                    self.entry_index += 1;
                    break Some(page_table_entry);
                } else {
                    // This entry points to another entry
                    self.parent_page_tables_entry_index_stack
                        .push(self.entry_index)
                        .unwrap();
                    self.entry_index = 0;
                    continue;
                }
            } else {
                let page_table_entry = PageTableEntry {
                    page_table_index_stack: get_page_table_index_stack(),
                    present: false,
                };
                self.entry_index += 1;
                break Some(page_table_entry);
            }
        }
    }
}
