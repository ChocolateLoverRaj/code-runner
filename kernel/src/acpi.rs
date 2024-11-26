use acpi::AcpiTables;
use anyhow::{anyhow, Context};

use crate::phys_mapper::PhysMapper;

pub unsafe fn init(
    rsdp_addr: usize,
    phys_mapper: PhysMapper,
) -> anyhow::Result<AcpiTables<PhysMapper>> {
    let acpi_tables = unsafe { AcpiTables::from_rsdp(phys_mapper, rsdp_addr) }
        .map_err(|e| anyhow!("{e:?}"))
        .context("Error reading ACPI tables")?;
    Ok(acpi_tables)
}
