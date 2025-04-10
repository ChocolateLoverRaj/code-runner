use core::{fmt::Write, ptr::NonNull};

use acpi::{
    address::AddressSpace,
    spcr::{Spcr, SpcrInterfaceType},
};
use uart_16550::uart_16550::Uart16550;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{FrameAllocator, Mapper, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr,
};

use crate::{
    find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table, hhdm_offset::HhdmOffset,
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
};

fn get_16550_compatible_mmio(spcr: &Spcr) -> Option<(PhysAddr, usize)> {
    match spcr.interface_type() {
        SpcrInterfaceType::Full16550
        | SpcrInterfaceType::Full16450
        | SpcrInterfaceType::Generic16550 => Some(()),
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

/// Says hello world through the SPCR.
pub fn spcr_hello_world(
    spcr: &Spcr,
    hhdm_offset: HhdmOffset,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let (base_address, stride) = get_16550_compatible_mmio(spcr).unwrap();
    // Map the page
    // Assume that the base address is aligned
    if stride * 8 > 0x1000 {
        todo!();
    }
    let physical_frame = PhysFrame::<Size4KiB>::from_start_address(base_address).unwrap();
    let mut offset_page_table = get_offset_page_table(hhdm_offset);
    let page = find_contiguous_unused_virtual_memory(
        unsafe { PageTablesRecursiveIterator::new(hhdm_offset, Cr3::read().0, 256) },
        1,
    )
    .unwrap()
    .start;
    unsafe {
        offset_page_table.map_to(
            page,
            physical_frame,
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE
                | PageTableFlags::WRITE_THROUGH,
            frame_allocator,
        )
    }
    .unwrap()
    .flush();
    let mut uart = unsafe {
        uart_16550::mmio::new(
            NonNull::new(page.start_address().as_mut_ptr()).unwrap(),
            stride,
        )
    };
    // Baud rate for Chromebooks: 115200
    // TODO: Determine baud rate for computers that are not Chromebooks
    uart.init_with_dl(0x01, 0x00);
    uart.write_str("This message is just to show that SPCR is working. We will make a nicer logging integration later.").unwrap();
    loop {
        let number = uart.receive();
        writeln!(uart, "Received input from SPCR: {}", number).unwrap();
    }
}
