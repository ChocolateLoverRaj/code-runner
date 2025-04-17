use core::{fmt::Debug, ops::Deref};

#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct IoPermissionBitmap<const N: usize> {
    bitmap: [u8; N],
}

impl<const N: usize> IoPermissionBitmap<N> {
    pub const fn new_deny_all() -> Self {
        Self {
            bitmap: [u8::MAX; N],
        }
    }

    /// If this contains data about if this port is allowed or not
    pub fn contains_port_permission(&self, port: u16) -> bool {
        (port as usize) < N * 8
    }

    /// Panics if port is not contained in self
    pub fn is_port_allowed(&self, port: u16) -> bool {
        if !self.contains_port_permission(port) {
            panic!("Port {} is not contained in the bitmap", port);
        }

        let byte_index = (port / 8) as usize;
        let bit_index = port % 8;

        // Check if the specific bit is 0 (allowed) or 1 (denied)
        (self.bitmap[byte_index] >> bit_index) & 1 != 0
    }

    pub fn set_port_allowed(&mut self, port: u16, is_allowed: bool) {
        if !self.contains_port_permission(port) {
            panic!("Port {} is not contained in the bitmap", port);
        }

        let byte_index = (port / 8) as usize;
        let bit_index = port % 8;

        if is_allowed {
            // Clear the bit (set to 0) to allow the port
            self.bitmap[byte_index] &= !(1 << bit_index);
        } else {
            // Set the bit (set to 1) to deny the port
            self.bitmap[byte_index] |= 1 << bit_index;
        }
    }
}

impl<const N: usize> Debug for IoPermissionBitmap<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self == &Self::new_deny_all() {
            write!(f, "IoPermissionBitmap (All Ports Denied)")
        } else {
            write!(f, "IoPermissionBitmap (Some Ports Allowed)")
        }
    }
}

impl<const N: usize> Deref for IoPermissionBitmap<N> {
    type Target = [u8; N];

    fn deref(&self) -> &Self::Target {
        &self.bitmap
    }
}
