use core::ops::DerefMut;

use alloc::boxed::Box;
use conquer_once::noblock::OnceCell;
use x2apic::lapic::LocalApic;
use x86_64::structures::idt::{HandlerFunc, InterruptStackFrame};

type LocalApicGetter = Box<dyn Fn() -> Box<dyn DerefMut<Target = LocalApic>> + Send + Sync>;

static GETTER: OnceCell<LocalApicGetter> = OnceCell::uninit();

/// This is private so that the getter must be initialized before using
extern "x86-interrupt" fn logging_timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    log::info!("Timer interrupt");
    let mut local_apic = GETTER.try_get().unwrap()();
    unsafe { local_apic.end_of_interrupt() };
}

pub fn get_logging_timer_interrupt_handler(getter: LocalApicGetter) -> HandlerFunc {
    GETTER.try_init_once(|| getter).unwrap();
    logging_timer_interrupt_handler
}
