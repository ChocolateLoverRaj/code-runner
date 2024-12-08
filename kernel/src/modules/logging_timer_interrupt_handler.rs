use conquer_once::noblock::OnceCell;
use x86_64::structures::idt::{HandlerFunc, InterruptStackFrame};

use super::local_apic_getter::LocalApicGetter;

static GETTER: OnceCell<LocalApicGetter> = OnceCell::uninit();

/// This is private so that the getter must be initialized before using
extern "x86-interrupt" fn logging_timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    log::debug!("Timer interrupt");
    let mut local_apic = GETTER.try_get().unwrap()();
    unsafe { local_apic.end_of_interrupt() };
}

pub fn get_logging_timer_interrupt_handler(getter: LocalApicGetter) -> HandlerFunc {
    GETTER.try_init_once(|| getter).unwrap();
    logging_timer_interrupt_handler
}
