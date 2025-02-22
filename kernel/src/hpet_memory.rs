use core::mem::MaybeUninit;

use bitfield::bitfield;
use volatile::{
    access::{NoAccess, ReadOnly, ReadWrite},
    VolatileFieldAccess,
};

/// Based on:
/// - https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/software-developers-hpet-spec-1-0a.pdf
/// - https://wiki.osdev.org/HPET#HPET_registers
#[repr(C)]
#[derive(Debug, VolatileFieldAccess)]
pub struct HpetMemory {
    #[access(ReadOnly)]
    pub capabilities_and_id: HpetGeneralCapabilitiesAndIdRegister,
    #[access(NoAccess)]
    _reserved_008_00f: [MaybeUninit<u8>; 0x8],
    #[access(ReadWrite)]
    pub config: HpetGeneralConfigurationRegister,
    #[access(NoAccess)]
    _reserved_018_01f: [MaybeUninit<u8>; 0x8],
    #[access(ReadWrite)]
    pub interrupt_status: HpetGeneralInterruptStatusRegister,
    #[access(NoAccess)]
    _reserved_028_0ef: [MaybeUninit<u8>; 0xC8],
    /// Make sure that you enable the HPET first. This register increases monotonically. You can write to this if the HPET is halted. To get the actual amount of seconds you need to multiply this by the period.
    #[access(ReadWrite)]
    pub main_counter_value_register: u64,
    #[access(NoAccess)]
    _reserved_0f8_0ff: [MaybeUninit<u8>; 0x8],
    #[access(ReadWrite)]
    /// There is memory for 32 timers, but there are not always physically 32 timers. Check the number of timers before accessing a timer's memory.
    pub timers: [HpetTimerMemory; 32],
}

bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct HpetGeneralCapabilitiesAndIdRegister(u64);
    impl Debug;

    /// From the docs:
    /// `COUNTER_CLK_PERIOD`
    /// > Main Counter Tick Period: This read-only field indicates the period at which the counter increments in femtoseconds (10^-15 seconds). A value of 0 in this field is not permitted. The value in this field must be less than or equal to 05F5E100h (10^8 femptoseconds = 100 nanoseconds). The resolution must be in femptoseconds (rather than picoseconds) in order to achieve a resolution of 50 ppm.
    pub u32, get_counter_clk_period, _: 63, 32;
    /// From the docs:
    /// `VENDOR_ID`
    /// > This read-only field will be the same as what would be assigned if this logic was a PCI function.
    pub u16, get_vendor_id, _: 31, 16;
    /// From the docs:
    /// `LEG_RT_CAP`
    /// > LegacyReplacement Route Capable: If this bit is a 1, it indicates that the hardware supports the LegacyReplacement Interrupt Route option.
    pub bool, get_leg_rt_cap, _: 15;
    /// From the docs:
    /// `COUNT_SIZE_CAP`
    /// > Counter Size:
    /// > - This bit is a 0 to indicate that the main counter is 32 bits wide (and cannot operate in 64-bit mode).
    /// > - This bit is a 1 to indicate that the main counter is 64 bits wide (although this does not preclude it from being operated in a 32-bit mode).
    pub bool, get_count_size_cap, _: 13;
    /// From the docs:
    /// `NUM_TIM_CAP`
    /// > *Number of Timers:* This indicates the number of timers in this block. The number in this field indicates the last timer (i.e. if there are three timers, the value will be 02h, four timers will be 03h, five timers will be 04h, etc.).
    pub u8, get_num_tim_cap, _: 12, 8;
    /// From the docs:
    /// `REV_ID`
    /// > This indicates which revision of the function is implemented. The value must NOT be 00h.
    pub u8, get_rev_id, _: 7, 0;
}

bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct HpetGeneralConfigurationRegister(u64);
    impl Debug;

    /// From the docs:
    /// > **LegacyReplacement Route:**
    /// > - 0 – Doesn’t support **LegacyReplacement Route**
    /// > - 1 – Supports **LegacyReplacement Route**
    /// > If the ENABLE_CNF bit and the LEG_RT_CNF bit are both set, then the interrupts will be routed as follows:
    /// > Timer 0 will be routed to IRQ0 in Non-APIC or IRQ2 in the I/O APIC
    /// > Timer 1 will be routed to IRQ8 in Non-APIC or IRQ8 in the I/O APIC
    /// > Timer 2-n will be routed as per the routing in the timer n config registers.
    /// >
    /// > If the LegacyReplacement Route bit is set, the individual routing bits for timers 0 and 1 (APIC or FSB) will have no impact.
    /// >
    /// > If the LegacyReplacement Route bit is not set, the individual routing bits for each of the timers are used.
    pub bool, get_legacy_replacement_cnf, set_legacy_replacement_cnf: 1;
    /// From the docs:
    /// `ENABLE_CNF`
    /// > Overall Enable: This bit must be set to enable any of the timers to generate interrupts. If this bit is 0, then the main counter will halt (will not increment) and no interrupts will be caused by any of these timers.
    /// > - 0 – Halt main count and disable all timer interrupts
    /// > - 1 – allow main counter to run, and allow timer interrupts if enabled
    pub bool, get_enable_cnf, set_enable_cnf: 0;
}

bitfield! {
    /// General Interrupt Status Register
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct HpetGeneralInterruptStatusRegister(u64);
    impl Debug;

    /// `Tn_INT_STS` in the docs. Timer *n* Interrupt Active.
    ///
    /// If this timer is set to level-triggered mode: This bit will be set to `1` if the timer's interrupt is active. You can set this bit to `0` by writing `1` to it.
    ///
    /// If set to edge-triggered mode: Ignore this. Always write `0` to it if you write to it.
    pub get_t_n_int_sts, set_t_n_int_sts: 31, 0, 32;
}

#[repr(C)]
#[derive(Debug, VolatileFieldAccess)]
pub struct HpetTimerMemory {
    pub configuration_and_capability_register: TimerNConfigurationAndCapabilityRegister,
    pub comparator_register: u64,
    /// Has bit fields, but I didn't add them cuz I won't be using them.
    pub fsb_route_register: u64,
    _reserved: MaybeUninit<u64>,
}

bitfield! {
    /// Timer N Configuration and Capability Register
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct TimerNConfigurationAndCapabilityRegister(u64);
    impl Debug;

    /// `Tn_INT_ROUTE_CAP` in the docs. Each bit represents a IO APIC interrupt. If a bit is 1, that means that this timer supports sending interrupt to the corresponding IO APIC interrupt based on the bit index, where bit 0 is the rightmost.
    pub u32, get_int_route_cap, _: 63, 32;
    pub bool, get_fsp_int_del_cap, set_fsp_int_del_cap: 15;
    pub bool, get_fsp_en_cnf, set_fsp_en_cnf: 14;
    pub u8, get_int_route_cnf, set_int_route_cnf: 13, 9;
    pub bool, get_32_mode_cnf, set_32_mode_cnf: 8;
    pub bool, get_val_set_cnf, set_val_set_cnf: 6;
    pub bool, get_size_cap, _: 5;
    pub bool, get_per_int_cp, set_per_int_cp: 4;
    pub bool, get_type_cnf, set_type_cnf: 3;
    pub bool, get_int_enb_cnf, set_int_enb_cnf: 2;
    pub bool, get_int_type_cnf, set_int_type_cnf: 1;
}
