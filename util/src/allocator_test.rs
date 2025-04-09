use core::{
    alloc::Layout,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
};

pub trait TestableAllocator {
    fn allocate(&mut self, layout: Layout) -> Option<NonZeroUsize>;
}

#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry {
    pub huge: bool,
    pub physical_frame_start: usize,
}

pub trait PageTable {
    fn present_entries(&self) -> impl Iterator<Item = (usize, PageTableEntry)>;

    fn get_entry(&self, index: usize) -> Option<PageTableEntry>;
}

pub trait Memory {
    /// This function will have consistent behavior in test environments and real environments
    fn usable_physical_memory_regions(&self) -> impl Iterator<Item = Range<usize>>;

    /// In real environments, it will read the `Cr3` register
    fn get_current_top_level_page_table(&self) -> usize;

    /// In test environments, it will panick virtual address is not mapped or of it is not a page table.
    /// In real environments, it will unsafely assume that the virtual address is a page table.
    fn get_page_table(&self, page_table_virtual_address: usize) -> impl PageTable;

    /// In a test environment, this will *copy* the memory (so that we can have memory across page borders)
    /// In a real environment, this will just (`unsafe`ly) give you a slice
    fn get_bytes(&self, virtual_address_range: Range<usize>) -> impl Deref<Target = [u8]>;

    fn get_bytes_mut(&self, virtual_address_range: Range<usize>) -> impl DerefMut<Target = [u8]>;
}
