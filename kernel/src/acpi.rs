use acpi::{AcpiResult, AcpiTables};
use spinning_top::Spinlock;
use util::init_later::InitLater;

use crate::{acpi_handler_impl::AcpiHandlerImpl, hhdm_offset::HhdmOffset, rsdp_addr::RsdpAddr};

pub static ACPI_TABLES: InitLater<Spinlock<AcpiTables<AcpiHandlerImpl>>> = InitLater::uninit();

pub fn init(
    rsdp_addr: RsdpAddr,
    hhdm_offset: HhdmOffset,
) -> AcpiResult<&'static Spinlock<AcpiTables<AcpiHandlerImpl>>> {
    unsafe {
        acpi::AcpiTables::from_rsdp(
            AcpiHandlerImpl { hhdm_offset },
            u64::from(rsdp_addr) as usize,
        )
    }
    .map(|acpi_tables| ACPI_TABLES.try_init(Spinlock::new(acpi_tables)).unwrap())
}
