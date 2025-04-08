/// Extracts the page table indices and offset from a virtual address.
///
/// In x86_64 paging, a virtual address is divided into:
/// - Bits 0–11: Page offset
/// - Bits 12–20: L1 index (PT)
/// - Bits 21–29: L2 index (PD)
/// - Bits 30–38: L3 index (PDPT)
/// - Bits 39–47: L4 index (PML4)
///
/// Returns a tuple of (L4, L3, L2, L1, offset).
pub fn get_page_table_indices(virtual_address: usize) -> (usize, usize, usize, usize, usize) {
    const ENTRY_COUNT: usize = 512; // 2^9 entries per table
    const PAGE_OFFSET_BITS: usize = 12; // Bits 0–11 are the page offset
    const INDEX_BITS: usize = 9; // Each index uses 9 bits

    let l1_shift = PAGE_OFFSET_BITS;
    let l2_shift = l1_shift + INDEX_BITS;
    let l3_shift = l2_shift + INDEX_BITS;
    let l4_shift = l3_shift + INDEX_BITS;

    let mask = ENTRY_COUNT - 1; // 0x1FF, or 0b111111111

    let l1 = (virtual_address >> l1_shift) & mask;
    let l2 = (virtual_address >> l2_shift) & mask;
    let l3 = (virtual_address >> l3_shift) & mask;
    let l4 = (virtual_address >> l4_shift) & mask;

    let offset = virtual_address & ((1 << PAGE_OFFSET_BITS) - 1);

    (l4, l3, l2, l1, offset)
}
