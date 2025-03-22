use x86_64::{registers::control::Cr3, structures::paging::PageTable};

pub fn ensure_mem_is_higher_half(hhdm_offset: u64) {
    // Since we will split between user space in lower half and kernel in higher half, we need to make sure lower half is completely clear
    let max_entries = 512;
    let active_l4_pt = {
        let (active_l4, _cr3_flags) = Cr3::read();
        let active_l4_pt = (active_l4.start_address().as_u64() + hhdm_offset) as *const PageTable;
        unsafe { &*active_l4_pt }
    };
    for i in 0..max_entries / 2 {
        assert!(
            active_l4_pt[i].is_unused(),
            "Unexpected page table entry at index {} of active L4 page table",
            i
        );
    }
    log::info!("Made sure all mapped memory is in the higher half");
}
