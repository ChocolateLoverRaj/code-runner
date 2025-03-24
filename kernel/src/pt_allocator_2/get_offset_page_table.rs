use x86_64::{
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable},
    VirtAddr,
};

/// Get the offset page table from current Cr3 value and HHDM offset
pub fn get_offset_page_table<'a>(hhdm_offset: u64) -> OffsetPageTable<'a> {
    unsafe {
        OffsetPageTable::new(
            {
                let (active_l4, _cr3_flags) = Cr3::read();
                let active_l4_pt =
                    (active_l4.start_address().as_u64() + hhdm_offset) as *mut PageTable;
                &mut *active_l4_pt
            },
            VirtAddr::new(hhdm_offset),
        )
    }
}
