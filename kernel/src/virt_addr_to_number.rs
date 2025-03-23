use x86_64::VirtAddr;

pub trait VirtAddrToNumber {
    /// Convert a `VirtAddr` into a `usize`, un-canonicalizing it
    fn into_number(self) -> usize;
}

impl VirtAddrToNumber for VirtAddr {
    fn into_number(self) -> usize {
        (self.as_u64() & 0x0000_FFFF_FFFF_FFFF) as usize
    }
}
