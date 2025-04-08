/// Constructs a virtual address from page table indices and offset.
///
/// This is the inverse of `get_page_table_indices`.
/// It does **not** perform canonicalization (i.e., bits 48–63 are not sign-extended).
///
/// Input:
/// - L4: PML4 index (bits 39–47)
/// - L3: PDPT index (bits 30–38)
/// - L2: PD index (bits 21–29)
/// - L1: PT index (bits 12–20)
/// - offset: page offset (bits 0–11)
///
/// Returns:
/// - The virtual address constructed from the input.
pub fn virtual_address_from_parts(
    l4: usize,
    l3: usize,
    l2: usize,
    l1: usize,
    offset: usize,
) -> usize {
    const INDEX_BITS: usize = 9;
    const PAGE_OFFSET_BITS: usize = 12;

    (l4 << (PAGE_OFFSET_BITS + 3 * INDEX_BITS))
        | (l3 << (PAGE_OFFSET_BITS + 2 * INDEX_BITS))
        | (l2 << (PAGE_OFFSET_BITS + 1 * INDEX_BITS))
        | (l1 << PAGE_OFFSET_BITS)
        | offset
}
