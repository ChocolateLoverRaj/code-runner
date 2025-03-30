/// All values are in bytes
#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryUsageStats {
    /// The amount of memory used by the page tables
    pub page_tables: usize,
    /// The amount of memory used by the allocator to store metadata for the allocator itself
    pub global_allocator_metadata: usize,
    /// Data allocated by the kernel's global allocator
    pub global_allocations: usize,
}
