use core::{cell::RefCell, ops::DerefMut, ptr::NonNull};

use acpi::{AcpiHandler, AcpiTables, HpetInfo};
use spinning_top::RwSpinlock;
use util::init_later::InitLater;
use volatile::VolatileRef;
use x2apic::ioapic::RedirectionTableEntry;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{FrameAllocator, Mapper, PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr,
};

use crate::{
    find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset,
    hpet_memory::{HpetMemory, HpetMemoryVolatileFieldAccess, HpetTimerMemoryVolatileFieldAccess},
    interrupt_numbers::InterruptNumbers,
    mapped_apics::MAPPED_APICS,
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
    pic8259_interrupts::Pic8259Interrupts,
};

pub static HPET: InitLater<RwSpinlock<VolatileRef<HpetMemory>>> = InitLater::uninit();

pub fn init(
    acpi_tables: &AcpiTables<impl AcpiHandler>,
    frame_allocator: &RefCell<impl FrameAllocator<Size4KiB>>,
    hhdm_offset: HhdmOffset,
) {
    let hpet_info = HpetInfo::new(acpi_tables).unwrap();
    log::info!("Hpet info: {:#?}", hpet_info);
    let mut o = get_offset_page_table(hhdm_offset);

    // The HPET is not guaranteed to be a multiple of and aligned to 4KiB
    let phys_start = PhysAddr::new(hpet_info.base_address as u64);
    let phys_end = phys_start + size_of::<HpetMemory>() as u64;
    let frame_count =
        phys_end.as_u64().div_ceil(Size4KiB::SIZE) - phys_start.as_u64().div_floor(Size4KiB::SIZE);
    let page_range = find_contiguous_unused_virtual_memory(
        unsafe { PageTablesRecursiveIterator::new(hhdm_offset, Cr3::read().0, 256) },
        frame_count,
    )
    .unwrap();
    let start_frame =
        PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(hpet_info.base_address as u64));
    for (index, page) in page_range.clone().enumerate() {
        unsafe {
            o.map_to(
                page,
                start_frame + index as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::NO_CACHE
                    | PageTableFlags::NO_EXECUTE,
                frame_allocator.borrow_mut().deref_mut(),
            )
        }
        .unwrap()
        .flush();
    }
    // Safety: The pointer is pointing to the start address of the HPET and we will never unmap the pages
    let mut hpet_volatile_ref = unsafe {
        VolatileRef::<HpetMemory>::new(
            NonNull::new(
                (page_range.start.start_address() + phys_start.as_u64() % Size4KiB::SIZE)
                    .as_mut_ptr(),
            )
            .unwrap(),
        )
    };
    let period_femto_seconds = hpet_volatile_ref
        .as_ptr()
        .capabilities_and_id()
        .read()
        .get_counter_clk_period();
    let timer_count = hpet_volatile_ref
        .as_mut_ptr()
        .capabilities_and_id()
        .read()
        .get_num_tim_cap()
        + 1;
    log::debug!(
        "Period: {} * 10^-15 s, Number of timers: {}",
        period_femto_seconds,
        timer_count
    );
    if timer_count < 1 {
        panic!("HPET doesn't have any timers for some reason");
    }

    for timer_index in 0..timer_count {
        let timer = hpet_volatile_ref
            .as_mut_ptr()
            .timers()
            .as_slice()
            .index(timer_index as usize);
        timer
            .comparator_register()
            .write((2 + timer_index as u64) * 1_000_000_000_000_000 / period_femto_seconds as u64);
        let mut timer_conf = timer.configuration_and_capability_register().read();
        let first_route = {
            let mut i = 0_u8;
            loop {
                if i == 32 {
                    break None;
                }
                if i != Pic8259Interrupts::Keyboard.into()
                    && timer_conf.get_int_route_cap(i as usize)
                {
                    break Some(i);
                }
                i += 1;
            }
        }
        .expect("Timer 0 is not capable of sending any interrupts apparently");
        // let first_route = 20;
        log::info!("HPET timer route: {}", first_route);
        timer_conf.set_int_route_cnf(first_route);
        timer_conf.set_int_enb_cnf(true);
        timer_conf.set_int_type_cnf(true);
        timer_conf.set_type_cnf(false);
        timer
            .configuration_and_capability_register()
            .write(timer_conf);

        let entry = {
            let mut entry = RedirectionTableEntry::default();
            // entry.set_dest(0);
            entry.set_vector(InterruptNumbers::Hpet.into());
            entry
        };
        let mut io_apic = MAPPED_APICS.try_get().unwrap().io_apic.lock();
        unsafe {
            io_apic.set_table_entry(first_route, entry);
            io_apic.enable_irq(first_route);
        };
    }

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

    HPET.try_init(RwSpinlock::new(hpet_volatile_ref)).unwrap();
}
