use limine::response::MemoryMapResponse;

use crate::get_total_memory::{
    get_acpi_reclaimable_memory, get_bootloader_reclaimable_memory, get_kernel_memory,
    get_total_memory,
};

pub fn log_memory_usage(memory_map_response: &MemoryMapResponse) {
    log::info!(
        "Total memory: 0x{:X}",
        get_total_memory(memory_map_response)
    );
    log::info!(
        "Bootloader reclaimable memory: 0x{:X}",
        get_bootloader_reclaimable_memory(memory_map_response)
    );
    log::info!(
        "ACPI reclaimable memory: 0x{:X}",
        get_acpi_reclaimable_memory(memory_map_response)
    );
    log::info!("Used memory: 0x{:X}", get_kernel_memory());
}
