use core::{
    error::Error,
    fmt::Display,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

use futures_util::{task::AtomicWaker, Stream};
use x86_rtc::interrupts::DividerValue;

static GOT_INTERRUPT: AtomicBool = AtomicBool::new(false);
static WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the RTC interrupt handler
///
/// Must not block or allocate
pub(crate) fn handle_interrupt() {
    GOT_INTERRUPT.store(true, Ordering::Relaxed);
    WAKER.wake();
}

pub struct RtcStream {
    _rtc_lock: spin::MutexGuard<'static, ()>,
}

static RTC_IN_USE: spin::Mutex<()> = spin::Mutex::new(());

#[derive(Debug)]
pub struct RtcInUse;
impl Display for RtcInUse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "RTC is already being used")
    }
}
impl Error for RtcInUse {}

impl RtcStream {
    pub fn new(divider_value: DividerValue) -> Result<Self, RtcInUse> {
        match RTC_IN_USE.try_lock() {
            Some(rtc_lock) => {
                log::debug!("Enabling RTC interrupts");
                x86_rtc::interrupts::enable();
                x86_rtc::interrupts::set_divider_value(divider_value);
                Ok(Self {
                    _rtc_lock: rtc_lock,
                })
            }
            None => Err(RtcInUse),
        }
    }

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
