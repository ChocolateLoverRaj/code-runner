use num_enum::IntoPrimitive;

#[derive(Debug, IntoPrimitive)]
#[repr(u8)]
pub enum InterruptNumbers {
    LocalApicSpurious = 0x20,
    LocalApicTimer,
    LocalApicError,
    Keyboard,
    Hpet,
}
