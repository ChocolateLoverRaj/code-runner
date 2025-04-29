use core::{cell::RefCell, ops::DerefMut, ptr::NonNull};

use acpi::{AcpiHandler, AcpiTables, HpetInfo};
use volatile::VolatileRef;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{FrameAllocator, Mapper, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr,
};

use crate::{
    find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset,
    hpet_memory::{HpetMemory, HpetMemoryVolatileFieldAccess, HpetTimerMemoryVolatileFieldAccess},
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
};

pub fn init(
    acpi_tables: &AcpiTables<impl AcpiHandler>,
    frame_allocator: &RefCell<impl FrameAllocator<Size4KiB>>,
    hhdm_offset: HhdmOffset,
) {
    let hpet_info = HpetInfo::new(acpi_tables).unwrap();
    log::info!("Hpet info: {:#?}", hpet_info);
    let mut o = get_offset_page_table(hhdm_offset);
    let page = find_contiguous_unused_virtual_memory(
        unsafe { PageTablesRecursiveIterator::new(hhdm_offset, Cr3::read().0, 256) },
        1,
    )
    .unwrap()
    .start;
    let frame =
        PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(hpet_info.base_address as u64))
            .unwrap();
    unsafe {
        o.map_to(
            page,
            frame,
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE
                | PageTableFlags::NO_EXECUTE,
            frame_allocator.borrow_mut().deref_mut(),
        )
    }
    .unwrap()
    .flush();
    // Safety: The pointer is pointing to the start address of the HPET and we will never unmap the pages
    let mut hpet_volatile_ref = unsafe {
        VolatileRef::<HpetMemory>::new(NonNull::new(page.start_address().as_mut_ptr()).unwrap())
    };
    log::info!(
        "Number of timers: {}",
        hpet_volatile_ref
            .as_mut_ptr()
            .capabilities_and_id()
            .read()
            .get_num_tim_cap()
            + 1
    );
    let timer = hpet_volatile_ref.as_mut_ptr().timers().as_slice().index(0);
    let r = timer.configuration_and_capability_register().read();
    log::debug!("R: {:#?}", r);

    hpet_volatile_ref
        .as_mut_ptr()
        .main_counter_value_register()
        .write(0);
    hpet_volatile_ref
        .as_mut_ptr()
        .config()
        .update(|mut config| {
            config.set_enable_cnf(true);
            config
        });
    // loop {
    //     log::debug!(
    //         "Counter value: {:#?}",
    //         hpet_volatile_ref
    //             .as_ptr()
    //             .main_counter_value_register()
    //             .read()
    //     );
    // }
}
