use core::{arch::asm, num::NonZeroU64};

use crate::{check_pointer::check_pointer, hhdm_offset::HhdmOffset};

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
        let rbp = check_pointer(self.rbp, self.hhdm_offset).ok()?;
        // Safety: We are assuming that rbp is a valid pointer
        let next_rbp = unsafe { rbp.read() };
        // Safety: We are assuming that rbp + 1 (u64) is a valid pointer
        let instruction_pointer = check_pointer(unsafe { rbp.add(1) }, self.hhdm_offset).ok()?;
        let instruction_address = unsafe { instruction_pointer.read() };
        self.rbp = next_rbp as *const _;
        NonZeroU64::new(instruction_address)
    }
}
