use alloc::sync::Arc;
use chrono::DateTime;
use futures_util::{join, StreamExt};
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};
use x86_rtc::{interrupts::DividerValue, Rtc};

use crate::{
    change_stream::StreamChanges,
    modules::{async_keyboard::AsyncKeyboard, async_rtc::AsyncRtc},
    stream_with_initial::StreamWithInitial,
};

pub async fn demo_async(async_keyboard: &mut AsyncKeyboard, async_rtc: &mut AsyncRtc) {
    join!(
        async {
            let mut scancodes = async_keyboard.stream();
            let mut keyboard = Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore,
            );

            while let Some(scancode) = scancodes.next().await {
                if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                    log::info!("{key_event:?}");
                }
            }
        },
        {
            let rtc = Arc::new(Rtc::new());
            async_rtc
                .stream(DividerValue::new(15).unwrap())
                .with_initial(())
                .map(move |()| rtc.get_unix_timestamp())
                .changes()
                .for_each(|rtc_unix_timestamp| async move {
                    let now = DateTime::from_timestamp(rtc_unix_timestamp as i64, 0);
                    match now {
                        Some(now) => {
                            let now = now.to_rfc2822();
                            log::info!("Time (in UTC): {now}");
                        }
                        None => {
                            log::warn!("Invalid RTC time: {rtc_unix_timestamp}");
                        }
                    }
                })
        },
    );
}
