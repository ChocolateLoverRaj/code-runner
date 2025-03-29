use limine::{memory_map::EntryType, response::MemoryMapResponse};
use x86_64::VirtAddr;

use crate::hhdm_offset::HhdmOffset;

pub fn is_offset_mapped(
    memory_map_response: &MemoryMapResponse,
    hhdm_offset: HhdmOffset,
    virt_addr: VirtAddr,
) -> bool {
    if let Some(phys_addr_if_hddm) = virt_addr.as_u64().checked_sub(hhdm_offset.into()) {
        memory_map_response
            .entries()
            .iter()
            .filter(|entry| {
                [
                    EntryType::USABLE,
                    EntryType::BOOTLOADER_RECLAIMABLE,
                    EntryType::EXECUTABLE_AND_MODULES,
                    EntryType::FRAMEBUFFER,
                ]
                .contains(&entry.entry_type)
            })
            .any(|entry| {
                let start = entry.base;
                let end = entry.base + entry.length;
                (start..end).contains(&phys_addr_if_hddm)
            })
    } else {
        false
    }
}
