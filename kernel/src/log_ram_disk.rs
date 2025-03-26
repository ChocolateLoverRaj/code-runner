pub fn log_ram_disk(module_response: Option<&limine::response::ModuleResponse>) {
    let ram_disk = module_response.and_then(|response| response.modules().first());
    match ram_disk {
        Some(ram_disk) => {
            log::info!(
                "Got ram disk at {:?} with len {:?}",
                ram_disk.addr(),
                ram_disk.size()
            );
        }
        None => {}
    }
}
