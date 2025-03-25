pub fn log_cpu_info(mp_response: &limine::response::MpResponse) {
    log::info!("{} CPUs", mp_response.cpus().len());
}
