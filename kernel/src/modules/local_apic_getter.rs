use core::ops::DerefMut;

use alloc::boxed::Box;
use x2apic::lapic::LocalApic;

pub type BoxApic = Box<dyn DerefMut<Target = LocalApic>>;
pub type LocalApicGetter = Box<dyn Fn() -> BoxApic + Send + Sync>;
