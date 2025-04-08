use core::mem::MaybeUninit;

/// N must be a multiple of 0x1000
#[repr(C, align(0x1000))]
pub struct PreReservedPages<const N: usize> {
    pub bytes: [MaybeUninit<u8>; N],
}
