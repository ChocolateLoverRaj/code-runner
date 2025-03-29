use crate::rsdp_addr::RsdpAddr;

pub fn log_rsdp_addr(rsdp_addr: RsdpAddr) {
    log::debug!("RSDP Address: {:?}", rsdp_addr);
}
