use chrono::DateTime;

use crate::limine_requests::BOOT_TIME;

pub fn log_boot_time() {
    match BOOT_TIME.get_response() {
        Some(response) => {
            match DateTime::from_timestamp(response.timestamp().as_secs() as i64, 0) {
                Some(boot_time) => {
                    log::info!("Boot time: {:#?}", boot_time);
                }
                None => {
                    log::warn!("Invalid boot time provided by Limine");
                }
            }
        }
        None => {
            log::warn!("No boot time provided by Limine");
        }
    }
}
