pub fn log_ram_disk(module_response: Option<&limine::response::ModuleResponse>) {
    let ram_disk = module_response.and_then(|response| response.modules().first());
    if let Some(ram_disk) = ram_disk {
        log::info!(
            "Got ram disk at {:?} with len {:?}",
            ram_disk.addr(),
            ram_disk.size()
        );
    }
}
