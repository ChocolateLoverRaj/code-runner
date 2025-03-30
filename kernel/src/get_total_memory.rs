use limine::{memory_map::EntryType, response::MemoryMapResponse};

use crate::pt_allocator_2::{memory_usage_stats::MemoryUsageStats, MEMORY_USAGE_STATS};

pub fn get_total_memory(memory_map_response: &MemoryMapResponse) -> usize {
    memory_map_response
        .entries()
        .iter()
        .filter(|entry| {
            entry.entry_type == EntryType::USABLE
                || entry.entry_type == EntryType::BOOTLOADER_RECLAIMABLE
                || entry.entry_type == EntryType::ACPI_RECLAIMABLE
        })
        .map(|entry| entry.length as usize)
        .sum()
}

pub fn get_bootloader_reclaimable_memory(memory_map_response: &MemoryMapResponse) -> usize {
    memory_map_response
        .entries()
        .iter()
        .filter(|entry| entry.entry_type == EntryType::BOOTLOADER_RECLAIMABLE)
        .map(|entry| entry.length as usize)
        .sum()
}

pub fn get_acpi_reclaimable_memory(memory_map_response: &MemoryMapResponse) -> usize {
    memory_map_response
        .entries()
        .iter()
        .filter(|entry| entry.entry_type == EntryType::ACPI_RECLAIMABLE)
        .map(|entry| entry.length as usize)
        .sum()
}

/// Get memory that was free to use when the bootloader booted the kernel, but is now being used by the kernel
pub fn get_memory_usage_stats() -> MemoryUsageStats {
    *MEMORY_USAGE_STATS.try_get().unwrap().lock()
}
