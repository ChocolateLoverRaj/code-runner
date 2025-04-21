use core::ops::Range;

use thiserror::Error;
use x86_64::{
    structures::paging::{OffsetPageTable, PageOffset, PageTable, PageTableFlags, PageTableIndex},
    VirtAddr,
};

use crate::virt_addr_from_indexes::{
    virt_addr_from_indexes_1_gib, virt_addr_from_indexes_2_mib, virt_addr_from_indexes_4_kib,
};

#[derive(Debug)]
pub struct FlagsNotAllowedError {
    pub actual_flags: PageTableFlags,
    pub expected_flags: PageTableFlags,
}

#[derive(Debug, Error)]
pub enum CheckVirtAddrRangeError {
    #[error("The page table entries did not contain the required flags")]
    FlagsNotAllowed(FlagsNotAllowedError),
}
pub fn check_virt_addr_range(
    mut range: Range<VirtAddr>,
    o: &OffsetPageTable,
    required_flags: PageTableFlags,
) -> Result<(), CheckVirtAddrRangeError> {
    loop {
        if range.start >= range.end {
            break;
        }
        let l4 = o.level_4_table();
        let l4_entry = &l4[range.start.p4_index()];
        if !l4_entry.flags().contains(required_flags) {
            return Err(CheckVirtAddrRangeError::FlagsNotAllowed(
                FlagsNotAllowedError {
                    actual_flags: l4_entry.flags(),
                    expected_flags: required_flags,
                },
            ));
        }
        let l3 = unsafe { &*(o.phys_offset() + l4_entry.addr().as_u64()).as_ptr::<PageTable>() };
        let l3_entry = &l3[range.start.p3_index()];
        if !l3_entry.flags().contains(required_flags) {
            return Err(CheckVirtAddrRangeError::FlagsNotAllowed(
                FlagsNotAllowedError {
                    actual_flags: l3_entry.flags(),
                    expected_flags: required_flags,
                },
            ));
        }
        if l3_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            range.start = virt_addr_from_indexes_1_gib(
                range.start.p4_index(),
                PageTableIndex::new(u16::from(range.start.p3_index()) + 1),
                0,
            );
            continue;
        }
        let l2_entry =
            &unsafe { &*(o.phys_offset() + l3_entry.addr().as_u64()).as_ptr::<PageTable>() }
                [range.start.p2_index()];
        if !l2_entry.flags().contains(required_flags) {
            return Err(CheckVirtAddrRangeError::FlagsNotAllowed(
                FlagsNotAllowedError {
                    actual_flags: l2_entry.flags(),
                    expected_flags: required_flags,
                },
            ));
        }
        if l2_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            range.start = virt_addr_from_indexes_2_mib(
                range.start.p4_index(),
                range.start.p3_index(),
                PageTableIndex::new(u16::from(range.start.p2_index()) + 1),
                0,
            );
            continue;
        }
        let l1_entry =
            &unsafe { &*(o.phys_offset() + l2_entry.addr().as_u64()).as_ptr::<PageTable>() }
                [range.start.p1_index()];
        if !l1_entry.flags().contains(required_flags) {
            return Err(CheckVirtAddrRangeError::FlagsNotAllowed(
                FlagsNotAllowedError {
                    actual_flags: l1_entry.flags(),
                    expected_flags: required_flags,
                },
            ));
        }
        range.start = virt_addr_from_indexes_4_kib(
            range.start.p4_index(),
            range.start.p3_index(),
            range.start.p2_index(),
            PageTableIndex::new(u16::from(range.start.p1_index()) + 1),
            PageOffset::new(0),
        );
    }
    Ok(())
}

pub fn ptr_to_virt_addr_range<T>(ptr: *const T) -> Range<VirtAddr> {
    let start = VirtAddr::from_ptr(ptr);
    start..start + size_of::<T>() as u64
}
