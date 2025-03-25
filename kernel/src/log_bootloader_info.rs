use crate::limine_requests::LIMINE_BOOTLOADER_INFO_REQUEST;

pub fn log_bootloader_info() {
    let bootloader_info = LIMINE_BOOTLOADER_INFO_REQUEST
        .get_response()
        .expect("No Limine bootloader info");
    log::info!(
        "This kernel was loaded by bootloader {:?} version {:?}",
        bootloader_info.name(),
        bootloader_info.version()
    );
}
