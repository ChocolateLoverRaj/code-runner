use limine::{memory_map::EntryType, response::MemoryMapResponse};
use x86_64::{registers::control::Cr3, structures::paging::PageTableFlags};

use crate::allocator_test::{Memory, PageTable, PageTableEntry};

pub struct X86Memory {
    memory_regions: &'static MemoryMapResponse,
}

impl X86Memory {
    pub fn new(memory_regions: &'static MemoryMapResponse) -> Self {
        Self { memory_regions }
    }
}

impl Memory for X86Memory {
    fn usable_physical_memory_regions(&self) -> impl Iterator<Item = core::ops::Range<usize>> {
        self.memory_regions
            .entries()
            .iter()
            .filter(|entry| entry.entry_type == EntryType::USABLE)
            .map(|entry| {
                let start = entry.base as usize;
                start..start + entry.length as usize
            })
    }

    fn get_current_top_level_page_table(&self) -> usize {
        Cr3::read().0.start_address().as_u64() as usize
    }

    fn get_page_table(
        &self,
        page_table_virtual_address: usize,
    ) -> impl crate::allocator_test::PageTable {
        let page_table = unsafe {
            &mut *(page_table_virtual_address as *mut x86_64::structures::paging::PageTable)
        };
        X86PageTable { page_table }
    }

    fn get_bytes(
        &self,
        virtual_address_range: core::ops::Range<usize>,
    ) -> impl core::ops::Deref<Target = [u8]> {
        unsafe {
            core::slice::from_raw_parts(
                virtual_address_range.start as *const u8,
                virtual_address_range.len(),
            )
        }
    }

    fn get_bytes_mut(
        &self,
        virtual_address_range: core::ops::Range<usize>,
    ) -> impl core::ops::DerefMut<Target = [u8]> {
        unsafe {
            core::slice::from_raw_parts_mut(
                virtual_address_range.start as *mut u8,
                virtual_address_range.len(),
            )
        }
    }
}

struct X86PageTable {
    page_table: &'static mut x86_64::structures::paging::PageTable,
}

impl PageTable for X86PageTable {
    fn present_entries(
        &self,
    ) -> impl Iterator<Item = (usize, crate::allocator_test::PageTableEntry)> {
        self.page_table
            .iter()
            .enumerate()
            .filter(|(_index, entry)| {
                !entry.is_unused() && entry.flags().contains(PageTableFlags::PRESENT)
            })
            .map(|(index, entry)| {
                (
                    index,
                    PageTableEntry {
                        huge: entry.flags().contains(PageTableFlags::HUGE_PAGE),
                        physical_frame_start: entry.addr().as_u64() as usize,
                    },
                )
            })
    }

    fn get_entry(&self, index: usize) -> Option<PageTableEntry> {
        let entry = &self.page_table[index];
        if !entry.is_unused() && entry.flags().contains(PageTableFlags::PRESENT) {
            Some(PageTableEntry {
                huge: entry.flags().contains(PageTableFlags::HUGE_PAGE),
                physical_frame_start: entry.addr().as_u64() as usize,
            })
        } else {
            None
        }
    }
}
