use crate::{hpet_memory::HpetMemoryVolatileFieldAccess, HPET};

pub fn syscall_enable_hpet() -> u64 {
    HPET.try_get()
        .unwrap()
        .write()
        .as_mut_ptr()
        .config()
        .update(|mut config| {
            config.set_enable_cnf(true);
            config
        });
    Default::default()
}
