use conquer_once::noblock::OnceCell;
use spin::Mutex;
use x2apic::lapic::LocalApic;

pub static LOCAL_APIC: OnceCell<Mutex<LocalApic>> = OnceCell::uninit();

pub fn store(local_apic: LocalApic) {
    LOCAL_APIC
        .try_init_once(|| spin::Mutex::new(local_apic))
        .unwrap();
}
