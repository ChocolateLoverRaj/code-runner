use core::{arch::asm, num::NonZeroU64};

use x86_64::structures::paging::PageTableFlags;

use crate::{
    check_pointer::{check_virt_addr_range, ptr_to_virt_addr_range},
    get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset,
};

/// The iterator gives you the instruction instruction addresses
/// /// The iterator starts at the current function and goes up the call stack.
#[derive(Debug, Clone)]
pub struct CallStackIterator {
    hhdm_offset: HhdmOffset,
    rbp: *const u64,
}

impl CallStackIterator {
    /// # Safety
    /// The call stack must be valid (it must use `rbp` in a valid way).
    pub unsafe fn new(hhdm_offset: HhdmOffset) -> Self {
        let rbp;
        unsafe {
            asm!(
                "mov {}, rbp",
                out(reg) rbp,
            );
        };
        Self { hhdm_offset, rbp }
    }
}

impl Iterator for CallStackIterator {
    type Item = NonZeroU64;

    fn next(&mut self) -> Option<Self::Item> {
        check_virt_addr_range(
            ptr_to_virt_addr_range(self.rbp),
            &get_offset_page_table(self.hhdm_offset),
            PageTableFlags::PRESENT,
        )
        .ok()?;
        // Safety: We are assuming that rbp is a valid pointer
        let next_rbp = unsafe { self.rbp.read() };
        // Safety: We are assuming that rbp + 1 (u64) is a valid pointer
        let instruction_pointer = unsafe { self.rbp.add(1) };
        check_virt_addr_range(
            ptr_to_virt_addr_range(instruction_pointer),
            &get_offset_page_table(self.hhdm_offset),
            PageTableFlags::PRESENT,
        )
        .ok()?;
        let instruction_address = unsafe { instruction_pointer.read() };
        self.rbp = next_rbp as *const _;
        NonZeroU64::new(instruction_address)
    }
}
