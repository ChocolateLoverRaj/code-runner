use alloc::sync::Arc;
use chrono::DateTime;
use futures_util::StreamExt;
use util::{change_stream::StreamChanges, stream_with_initial::StreamWithInitial};
use x86_rtc::{interrupts::DividerValue, Rtc};

use crate::modules::async_rtc::AsyncRtc;

pub async fn demo_asyc_rtc_drop(async_rtc: &mut AsyncRtc) {
    let rtc = Arc::new(Rtc::new());
    loop {
        async_rtc
            .stream(DividerValue::new(15).unwrap())
            .with_initial(())
            .map({
                let rtc = rtc.clone();
                move |()| rtc.get_unix_timestamp()
            })
            .changes()
            .take(5)
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
            .await;
    }
}
