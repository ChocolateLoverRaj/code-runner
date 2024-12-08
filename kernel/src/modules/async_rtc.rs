use conquer_once::noblock::OnceCell;
use x2apic::ioapic::{IoApic, RedirectionTableEntry};
use x86_64::structures::idt::{self, HandlerFunc, InterruptStackFrame};
use x86_rtc::interrupts::read_register_c;

use core::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use futures_util::{task::AtomicWaker, Stream};
use x86_rtc::interrupts::DividerValue;

use crate::pic8259_interrupts::Pic8259Interrupts;

use super::{idt::IdtBuilder, local_apic_getter::LocalApicGetter};

static GOT_INTERRUPT: AtomicBool = AtomicBool::new(false);
static WAKER: AtomicWaker = AtomicWaker::new();
static GETTER: OnceCell<LocalApicGetter> = OnceCell::uninit();

pub extern "x86-interrupt" fn rtc_interrupt_handler(_stack_frame: InterruptStackFrame) {
    GOT_INTERRUPT.store(true, Ordering::Relaxed);
    WAKER.wake();
    read_register_c();
    let mut local_apic = GETTER.try_get().unwrap()();
    unsafe { local_apic.end_of_interrupt() };
}

pub struct AsyncRtcBuilder {
    interrupt_index: u8,
}

impl AsyncRtcBuilder {
    pub fn set_interrupt(idt_builder: &mut IdtBuilder) -> Option<Self> {
        let interrupt_index = idt_builder.set_flexible_entry({
            let mut entry = idt::Entry::<HandlerFunc>::missing();
            entry.set_handler_fn(rtc_interrupt_handler);
            entry
        })?;
        Some(Self { interrupt_index })
    }

    pub fn configure_io_apic(
        &'static self,
        io_apic: &mut IoApic,
        getter: LocalApicGetter,
    ) -> AsyncRtc {
        GETTER.try_init_once(|| getter).unwrap();
        unsafe {
            io_apic.set_table_entry(Pic8259Interrupts::Rtc.into(), {
                let mut entry = RedirectionTableEntry::default();
                entry.set_vector(self.interrupt_index);
                entry
            })
        };
        unsafe { io_apic.enable_irq(Pic8259Interrupts::Rtc.into()) };
        AsyncRtc {}
    }
}

pub struct AsyncRtc {}

impl AsyncRtc {
    pub fn stream(&mut self, divider_value: DividerValue) -> RtcStream {
        log::debug!("Enabling RTC interrupts");
        x86_rtc::interrupts::enable();
        x86_rtc::interrupts::set_divider_value(divider_value);
        RtcStream {}
    }
}

pub struct RtcStream {}

impl RtcStream {
    pub fn set_divider_value(&mut self, divider_value: DividerValue) {
        x86_rtc::interrupts::set_divider_value(divider_value);
    }
}

impl Drop for RtcStream {
    fn drop(&mut self) {
        log::debug!("Disabling RTC interrupts");
        x86_rtc::interrupts::disable();
    }
}

impl Stream for RtcStream {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        WAKER.register(&cx.waker());
        match GOT_INTERRUPT.swap(false, Ordering::Relaxed) {
            true => Poll::Ready(Some(())),
            false => Poll::Pending,
        }
    }
}
