use core::ptr::NonNull;

use acpi::{
    address::AddressSpace,
    spcr::{Spcr, SpcrInteraceType},
    AcpiHandler, AcpiTables,
};
use alloc::boxed::Box;
use uart_16550_2::uart_16550::Uart16550;
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr,
};

use crate::{
    dynamic_combined_logger::DynamicLogger, logger::replace_serial_logger,
    virt_mem_tracker::VirtMemTracker, write_logger::LockedWriteLogger, write_with_cr::WriterWithCr,
};

/// Get the physical base address and stride that you can use with the `uart_16550` crate. If the console is not 16550 compatible or isn't accessed through physical memory, returns `None`.
fn get_16550_compatible_mmio(spcr: &Spcr) -> Option<(PhysAddr, usize)> {
    match spcr.interface_type() {
        SpcrInteraceType::Full16550
        | SpcrInteraceType::Full16450
        | SpcrInteraceType::Generic16550 => Some(()),
        _ => None,
    }?;
    let base_address = spcr.base_address()?.ok()?;
    match base_address.address_space {
        AddressSpace::SystemMemory => Some(()),
        _ => None,
    }?;
    match base_address.bit_offset {
        0 => Some(()),
        _ => None,
    }?;
    Some((
        PhysAddr::new(base_address.address),
        (base_address.bit_width / 8) as usize,
    ))
}

/// If a SPCR ACPI table is present, it means that instead of using COM1 as a serial port we need to do something else to access the serial port.
/// **This function allocates.**
pub fn replace_serial_logger_if_redirected(
    acpi_tables: &AcpiTables<impl AcpiHandler>,
    virt_mem_tracker: &mut VirtMemTracker,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), ()> {
    if let Ok(spcr) = acpi_tables.find_table::<Spcr>() {
        log::info!("SPCR Found");
        match get_16550_compatible_mmio(&spcr) {
            Some((base_addr, stride)) => {
                log::info!("This computer uses a MMIO 16550 compatible interface instead of COM1. If the logger is switched goes successfully, this is the last message you will see here.");
                // We assume it will only take up to 4096 bytes
                let aligned_down_phys_addr = base_addr.align_down(Size4KiB::SIZE);
                let offset_in_page: u64 = base_addr - aligned_down_phys_addr;
                let page = virt_mem_tracker.allocate_pages::<Size4KiB>(1).unwrap();
                let frame = PhysFrame::from_start_address(aligned_down_phys_addr).unwrap();
                unsafe {
                    mapper
                        .map_to(
                            page,
                            frame,
                            PageTableFlags::PRESENT
                                | PageTableFlags::WRITABLE
                                | PageTableFlags::NO_CACHE
                                | PageTableFlags::WRITE_THROUGH,
                            frame_allocator,
                        )
                        .unwrap()
                        .flush();
                };
                let base_virt_addr = (page.start_address() + offset_in_page).as_mut_ptr();
                let mut uart = unsafe {
                    uart_16550_2::mmio::new(NonNull::new(base_virt_addr).unwrap(), stride)
                };
                // Baud rate for Chromebooks: 115200
                // TODO: Determine baud rate for computers that are not Chromebooks
                uart.init_with_dl(0x01, 0x00);

                replace_serial_logger(DynamicLogger::Heap(Box::new(LockedWriteLogger::new(
                    WriterWithCr::new(uart),
                ))));

                log::info!("Logging to memory-mapped 16550-compatible interface starting at physical address {:?} with a stride of {} bytes", base_addr, stride);

                Ok(())
            }
            None => Err(()),
        }
    } else {
        log::info!("No SPCR found. Will continue using COM1.");
        Ok(())
    }
}
