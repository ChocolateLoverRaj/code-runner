use crate::limine_requests::RSDP_REQUEST;

pub fn log_rsdp_addr() {
    let rsdp = RSDP_REQUEST.get_response().unwrap().address();
    log::debug!("RSDP Address: 0x{:X}", rsdp);
}
