use crate::{
    allocator_test::{Memory, PageTable},
    virtual_address_from_parts::virtual_address_from_parts,
};

#[derive(Debug)]
pub struct PageMapping {
    pub virt_start: usize,
    pub phys_start: usize,
    pub len: usize,
}

/// Recursively traverses page table, returning every mapping
pub struct PageTablesTraverser<'a, T: Memory> {
    memory: &'a T,
    top_level_page_table: usize,
    offset_mapped_start: usize,
    sub_pt_stack: heapless::Vec<usize, 3>,
    entry_index: usize,
}

impl<'a, T: Memory> PageTablesTraverser<'a, T> {
    pub fn new(memory: &'a T, top_level_page_table: usize, offset_mapped_start: usize) -> Self {
        Self {
            memory,
            top_level_page_table,
            offset_mapped_start,
            sub_pt_stack: Default::default(),
            entry_index: Default::default(),
        }
    }
}

impl<'a, T: Memory> Iterator for PageTablesTraverser<'a, T> {
    type Item = PageMapping;

    fn next(&mut self) -> Option<Self::Item> {
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
                let mut pt = self
                    .memory
                    .get_page_table(self.top_level_page_table + self.offset_mapped_start)
                    .unwrap();
                // Traverse the page tables until we get to the lowest level we want to process
                for index in &self.sub_pt_stack {
                    pt = self
                        .memory
                        .get_page_table(
                            pt.get_entry(*index).unwrap().physical_frame_start
                                + self.offset_mapped_start,
                        )
                        .unwrap();
                }
                pt
            };
            let entry = pt.get_entry(self.entry_index);
            if let Some(entry) = entry {
                if entry.huge || self.sub_pt_stack.len() == 3 {
                    // This entry point to a phys frame, which could be 4KiB, 2MiB, or 1GiB
                    // Note that just cuz PageTableFlags::HUGE_PAGE is 1 doesn't mean that it's >4KiB - see https://github.com/phil-opp/blog_os/issues/1403
                    let page_mapping = PageMapping {
                        virt_start: {
                            let mut indexes =
                                heapless::Vec::<_, 4>::from_slice(&self.sub_pt_stack).unwrap();
                            indexes.push(self.entry_index).unwrap();
                            virtual_address_from_parts(
                                indexes.get(0).copied().unwrap_or_default(),
                                indexes.get(1).copied().unwrap_or_default(),
                                indexes.get(2).copied().unwrap_or_default(),
                                indexes.get(3).copied().unwrap_or_default(),
                                0,
                            )
                        },
                        phys_start: entry.physical_frame_start,
                        len: 0x1000
                            * 512_usize.pow((3 - self.sub_pt_stack.len()).try_into().unwrap()),
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
