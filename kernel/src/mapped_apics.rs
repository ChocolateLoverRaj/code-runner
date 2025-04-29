use core::cell::RefCell;

use acpi::{AcpiHandler, AcpiTables};
use spinning_top::Spinlock;
use util::init_later::InitLater;
use x2apic::ioapic::IoApic;
use x86_64::{
    structures::paging::{FrameAllocator, Size4KiB},
    VirtAddr,
};

use crate::{
    hhdm_offset::HhdmOffset,
    map_local_xapic::{map_apics, LocalXapicVirtAddr, MappedApics},
};

pub struct ApicData {
    pub io_apic: Spinlock<IoApic>,
    pub local_xapic: Option<LocalXapicVirtAddr>,
}

pub static MAPPED_APICS: InitLater<ApicData> = InitLater::uninit();

pub fn init(
    acpi_tables: &AcpiTables<impl AcpiHandler>,
    hhdm_offset: HhdmOffset,
    frame_allocator: &RefCell<impl FrameAllocator<Size4KiB>>,
) {
    let MappedApics {
        io_apic,
        local_xapic,
    } = map_apics(acpi_tables, hhdm_offset, frame_allocator).unwrap();
    MAPPED_APICS
        .try_init(ApicData {
            io_apic: Spinlock::new(unsafe { IoApic::new(VirtAddr::from(io_apic).as_u64()) }),
            local_xapic,
        })
        .unwrap();
}
