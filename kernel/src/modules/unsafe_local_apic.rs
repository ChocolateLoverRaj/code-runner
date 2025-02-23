use core::ops::{Deref, DerefMut};

use x2apic::lapic::LocalApic;

pub struct UnsafeLocalApic(pub LocalApic);

unsafe impl Send for UnsafeLocalApic {}
unsafe impl Sync for UnsafeLocalApic {}

impl Deref for UnsafeLocalApic {
    type Target = LocalApic;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UnsafeLocalApic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
