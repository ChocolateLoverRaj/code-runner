use x86_64::structures::idt::InterruptStackFrame;

use crate::apic::LOCAL_APIC;

pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // log::info!("Timer interrupt");
    let mut local_apic = LOCAL_APIC.get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}
