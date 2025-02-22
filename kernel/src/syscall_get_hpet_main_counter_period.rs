use crate::{hpet_memory::HpetMemoryVolatileFieldAccess, HPET};

pub fn syscall_get_hpet_main_counter_period() -> u64 {
    HPET.try_get()
        .unwrap()
        .read()
        .as_ptr()
        .capabilities_and_id()
        .read()
        .get_counter_clk_period() as u64
}
