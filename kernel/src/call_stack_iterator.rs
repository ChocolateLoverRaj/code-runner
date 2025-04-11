use core::{arch::asm, num::NonZeroU64, ptr::NonNull};

/// The iterator gives you the instruction pointers.
/// The iterator starts at the current function and goes up the call stack.
#[derive(Debug, Clone)]
pub struct CallStackIterator {
    rbp: *const u64,
}

impl CallStackIterator {
    /// # Safety
    /// The call stack must be valid (it must use `rbp` in a valid way).
    pub unsafe fn new() -> Self {
        let rbp;
        unsafe {
            asm!(
                "mov {}, rbp",
                out(reg) rbp,
            );
        };
        Self { rbp }
    }
}

impl Iterator for CallStackIterator {
    type Item = NonZeroU64;

    fn next(&mut self) -> Option<Self::Item> {
        let rbp = NonNull::new(self.rbp.cast_mut())?;
        // Safety: We are assuming that rbp is a valid pointer
        let next_rbp = unsafe { rbp.read() };
        // Safety: We are assuming that rbp + 1 (u64) is a valid pointer
        let instruction_pointer = unsafe { (rbp.add(1)).read() };
        self.rbp = next_rbp as *const _;
        NonZeroU64::new(instruction_pointer)
    }
}
