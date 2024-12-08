use acpi::{platform::interrupt::Apic, AcpiHandler, AcpiTables};
use alloc::alloc::Global;
use anyhow::anyhow;

pub fn get_apic<H: AcpiHandler>(acpi_tables: &AcpiTables<H>) -> anyhow::Result<Apic<Global>> {
    let platform_info = acpi_tables.platform_info().map_err(|e| anyhow!("{e:?}"))?;
    match platform_info.interrupt_model {
        acpi::InterruptModel::Apic(apic) => Ok(apic),
        interrupt_model => Err(anyhow!("Other interrupt model: {interrupt_model:#?}")),
    }
}
