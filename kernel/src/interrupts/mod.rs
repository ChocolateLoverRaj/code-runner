use num_enum::IntoPrimitive;

pub mod keyboard;
pub mod rtc;

#[derive(Debug, Clone, Copy, IntoPrimitive)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,
    Keyboard,
    LocalApicError,
    Rtc,
}
