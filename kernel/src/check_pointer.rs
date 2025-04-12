use thiserror::Error;
use x86_64::{
    structures::paging::{mapper::TranslateResult, Page, PageTableFlags, Size4KiB, Translate},
    PhysAddr, VirtAddr,
};

use crate::{get_offset_page_table::get_offset_page_table, hhdm_offset::HhdmOffset};

#[derive(Debug, Error)]
pub enum CheckPointerError {
    #[error("The pointer is null. Rust doesn't allow reading null pointers")]
    IsNull,
    #[error("The pointer is not aligned")]
    IsNotAligned,
    #[error("The page table flags doesn't contain PRESENT")]
    NoPresentFlag,
    #[error("The virtual memory is not mapped")]
    NotMapped,
    #[error("The frame address is invalid")]
    InvalidFrameAddress(PhysAddr),
}

/// Checks if a pointer will cause a page fault
pub fn check_pointer<T>(
    ptr: *const T,
    hhdm_offset: HhdmOffset,
) -> Result<*const T, CheckPointerError> {
    if ptr.is_null() {
        return Err(CheckPointerError::IsNull);
    }
    if !ptr.is_aligned() {
        return Err(CheckPointerError::IsNotAligned);
    }
    let page_range = {
        let start_page = Page::<Size4KiB>::containing_address(VirtAddr::from_ptr(ptr));
        let end_page =
            Page::<Size4KiB>::containing_address(VirtAddr::from_ptr(unsafe { ptr.add(1) }) - 1);
        start_page..=end_page
    };
    let page_table = get_offset_page_table(hhdm_offset);
    for page in page_range {
        match page_table.translate(page.start_address()) {
            TranslateResult::Mapped {
                frame: _,
                offset: _,
                flags,
            } => {
                if !flags.contains(PageTableFlags::PRESENT) {
                    return Err(CheckPointerError::NoPresentFlag);
                }
            }
            TranslateResult::NotMapped => {
                return Err(CheckPointerError::NotMapped);
            }
            TranslateResult::InvalidFrameAddress(address) => {
                return Err(CheckPointerError::InvalidFrameAddress(address))
            }
        };
    }
    Ok(ptr)
}
