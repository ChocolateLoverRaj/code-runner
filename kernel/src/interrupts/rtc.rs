use x86_64::structures::idt::InterruptStackFrame;
use x86_rtc::interrupts::read_register_c;

use crate::{apic::LOCAL_APIC, task::rtc::handle_interrupt};

pub extern "x86-interrupt" fn rtc_interrupt_handler(_stack_frame: InterruptStackFrame) {
    handle_interrupt();
    read_register_c();
    let mut local_apic = LOCAL_APIC.get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}
