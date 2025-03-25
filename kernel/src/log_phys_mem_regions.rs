use core::mem;

use crate::limine_requests::MEMORY_MAP_REQUEST;

pub fn log_phys_mem_regions() {
    let memory_map_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    memory_map_response.entries().iter().for_each(|entry| {
        log::debug!(
            "Memory ({:?}) at 0x{:013X?}..0x{:013X}",
            unsafe { mem::transmute::<_, u64>(entry.entry_type) },
            entry.base,
            entry.base + entry.length
        );
    });
}
