use x86_64::{
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable, PhysFrame},
    VirtAddr,
};

use crate::hhdm_offset::HhdmOffset;

/// Get the offset page table from current Cr3 value and HHDM offset
pub fn get_offset_page_table<'a>(hhdm_offset: HhdmOffset) -> OffsetPageTable<'a> {
    unsafe {
        OffsetPageTable::new(
            {
                let (active_l4, _cr3_flags) = Cr3::read();
                let active_l4_pt =
                    (active_l4.start_address().as_u64() + u64::from(hhdm_offset)) as *mut PageTable;
                &mut *active_l4_pt
            },
            VirtAddr::new(hhdm_offset.into()),
        )
    }
}

/// Get the offset page table from a specified L4 page table and HHDM offset
pub fn get_offset_page_table_with_new_l4<'a>(
    l4: PhysFrame,
    hhdm_offset: HhdmOffset,
) -> OffsetPageTable<'a> {
    unsafe {
        OffsetPageTable::new(
            {
                let active_l4_pt =
                    (l4.start_address().as_u64() + u64::from(hhdm_offset)) as *mut PageTable;
                active_l4_pt.write(Default::default());
                &mut *active_l4_pt
            },
            VirtAddr::new(hhdm_offset.into()),
        )
    }
}
