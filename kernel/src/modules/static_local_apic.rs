use alloc::boxed::Box;
use conquer_once::noblock::OnceCell;
use spin::Mutex;
use x2apic::lapic::LocalApic;

use super::local_apic_getter::{BoxApic, LocalApicGetter};

static LOCAL_APIC: OnceCell<Mutex<LocalApic>> = OnceCell::uninit();

pub fn get_getter() -> LocalApicGetter {
    Box::new(|| -> BoxApic { Box::new(LOCAL_APIC.try_get().unwrap().try_lock().unwrap()) })
}

pub fn enable_and_store(mut local_apic: LocalApic) {
    unsafe { local_apic.enable() };
    LOCAL_APIC
        .try_init_once(|| spin::Mutex::new(local_apic))
        .unwrap();
}
