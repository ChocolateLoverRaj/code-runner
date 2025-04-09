use limine::response::MemoryMapResponse;
use util::{
    paging_allocator::{scan_memory, PagingAllocator},
    x86_memory::x86_memory::X86Memory,
};

use crate::hhdm_offset::HhdmOffset;

pub fn init_2(memory: &'static MemoryMapResponse, hhdm_offset: HhdmOffset) {
    let (a, b) = scan_memory(&X86Memory::new(memory), u64::from(hhdm_offset) as usize);
    log::info!("Phys: {:#?}. Virt: {:#?}", a, b);
}
