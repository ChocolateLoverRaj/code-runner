use crate::{hpet_memory::HpetMemoryVolatileFieldAccess, HPET};

pub fn syscall_hpet_read_main_counter_value() -> u64 {
    HPET.try_get()
        .unwrap()
        .read()
        .as_ptr()
        .main_counter_value_register()
        .read()
}
