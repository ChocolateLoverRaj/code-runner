use x86_64::{
    structures::paging::{PageOffset, PageTableIndex},
    VirtAddr,
};

/// Get a virtual address that points to a 4KiB frame
pub fn virt_addr_from_indexes_4_kib(
    p4_index: PageTableIndex,
    p3_index: PageTableIndex,
    p2_index: PageTableIndex,
    p1_index: PageTableIndex,
    page_offset: PageOffset,
) -> VirtAddr {
    VirtAddr::new_truncate({
        (u64::from(p4_index) << 12 << 9 << 9 << 9)
            + (u64::from(p3_index) << 12 << 9 << 9)
            + (u64::from(p2_index) << 12 << 9)
            + (u64::from(p1_index) << 12)
            + u64::from(page_offset)
    })
}

pub fn test_virt_addr_from_indexes_4_kib() {
    let info = {
        let virt_addr = virt_addr_from_indexes_4_kib(
            PageTableIndex::new(1),
            PageTableIndex::new(2),
            PageTableIndex::new(3),
            PageTableIndex::new(5),
            PageOffset::new(7),
        );
        (
            virt_addr.p4_index(),
            virt_addr.p3_index(),
            virt_addr.p2_index(),
            virt_addr.p1_index(),
            virt_addr.page_offset(),
        )
    };
    log::info!("{:?}", info);
}

/// Get a virtual address that point to a 2MiB physical region of 512 frames
pub fn virt_addr_from_indexes_2_mib(
    p4_index: PageTableIndex,
    p3_index: PageTableIndex,
    p2_index: PageTableIndex,
    page_offset: u32,
) -> VirtAddr {
    let page_offset_valid = page_offset < (1 << 12 << 9);
    assert!(page_offset_valid, "Page offset is too big");
    VirtAddr::new_truncate({
        (u64::from(p4_index) << 12 << 9 << 9 << 9)
            + (u64::from(p3_index) << 12 << 9 << 9)
            + (u64::from(p2_index) << 12 << 9)
            + u64::from(page_offset)
    })
}

pub fn test_virt_addr_from_indexes_2_mib() {
    let info = {
        let virt_addr = virt_addr_from_indexes_2_mib(
            PageTableIndex::new(1),
            PageTableIndex::new(2),
            PageTableIndex::new(3),
            0,
        );
        (
            virt_addr.p4_index(),
            virt_addr.p3_index(),
            virt_addr.p2_index(),
            virt_addr.page_offset(),
        )
    };
    log::info!("{:?}", info);
}

/// Get a virtual address that point to a 1GiB physical region of 512 * 512 frames
pub fn virt_addr_from_indexes_1_gib(
    p4_index: PageTableIndex,
    p3_index: PageTableIndex,
    page_offset: u32,
) -> VirtAddr {
    let page_offset_valid = page_offset < (1 << 12 << 9 << 9);
    assert!(page_offset_valid, "Page offset is too big");
    VirtAddr::new_truncate({
        (u64::from(p4_index) << 12 << 9 << 9 << 9)
            + (u64::from(p3_index) << 12 << 9 << 9)
            + u64::from(page_offset)
    })
}

pub fn test_virt_addr_from_indexes_1_gib() {
    let info = {
        let virt_addr =
            virt_addr_from_indexes_1_gib(PageTableIndex::new(1), PageTableIndex::new(2), 0);
        (
            virt_addr.p4_index(),
            virt_addr.p3_index(),
            virt_addr.page_offset(),
        )
    };
    log::info!("{:?}", info);
}
