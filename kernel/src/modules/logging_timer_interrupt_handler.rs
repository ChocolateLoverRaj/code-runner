use conquer_once::noblock::OnceCell;
use spin::Mutex;
use x2apic::lapic::LocalApic;
use x86_64::structures::idt::{HandlerFunc, InterruptStackFrame};

use super::unsafe_local_apic::UnsafeLocalApic;

static LOCAL_APIC: OnceCell<&'static OnceCell<Mutex<UnsafeLocalApic>>> = OnceCell::uninit();

/// This is private so that the getter must be initialized before using
extern "x86-interrupt" fn logging_timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    log::debug!("Timer interrupt");
    let mut local_apic = LOCAL_APIC.try_get().unwrap().try_get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}

pub fn get_logging_timer_interrupt_handler(
    local_apic: &'static OnceCell<Mutex<UnsafeLocalApic>>,
) -> HandlerFunc {
    LOCAL_APIC.try_init_once(|| local_apic).unwrap();
    logging_timer_interrupt_handler
}
