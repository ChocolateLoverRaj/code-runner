use num_enum::IntoPrimitive;

#[derive(IntoPrimitive)]
#[repr(u8)]
pub enum Pic8259Interrupts {
    Timer,
    Keyboard,
    Rtc = 8,
}
