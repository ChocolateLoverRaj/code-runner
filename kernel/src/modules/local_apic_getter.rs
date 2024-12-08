use core::ops::DerefMut;

use alloc::boxed::Box;
use x2apic::lapic::LocalApic;

pub type LocalApicGetter = Box<dyn Fn() -> Box<dyn DerefMut<Target = LocalApic>> + Send + Sync>;
