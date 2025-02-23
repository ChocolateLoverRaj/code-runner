use conquer_once::noblock::OnceCell;
use spin::Mutex;

use super::unsafe_local_apic::UnsafeLocalApic;

pub static LOCAL_APIC: OnceCell<Mutex<UnsafeLocalApic>> = OnceCell::uninit();

pub fn store(local_apic: UnsafeLocalApic) {
    LOCAL_APIC
        .try_init_once(|| spin::Mutex::new(local_apic))
        .unwrap();
}
