#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[macro_use]
extern crate alloc;

use core::{
    cmp::Ordering,
    ops::{DerefMut, Range},
    panic::PanicInfo,
};

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use allocator::Dummy;
use apic::init_apic;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use embedded_graphics::{
    image::Image,
    pixelcolor::Rgb888,
    prelude::{Drawable, Point, Primitive, Size, Transform},
    primitives::{PrimitiveStyle, Rectangle},
};
use find_used_virt_addrs::find_used_virt_addrs;
use framebuffer::Display;
use gtd::init_gtd;
use hlt_loop::hlt_loop;
use interrupts::init_interrupts;
use logger::init_logger_with_framebuffer;
use tinytga::Tga;
use virt_addr_from_indexes::{
    test_virt_addr_from_indexes_1_gib, test_virt_addr_from_indexes_2_mib,
    test_virt_addr_from_indexes_4_kib, virt_addr_from_indexes_1_gib, virt_addr_from_indexes_2_mib,
    virt_addr_from_indexes_4_kib,
};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        page_table::FrameError, MappedPageTable, Page, PageOffset, PageSize, PageTable,
        PageTableFlags, PageTableIndex, PhysFrame, Size1GiB, Size2MiB, Size4KiB, Translate,
    },
    VirtAddr,
};

pub mod allocator;
pub mod apic;
pub mod find_used_virt_addrs;
pub mod framebuffer;
pub mod gtd;
pub mod hlt_loop;
pub mod interrupts;
pub mod logger;
pub mod memory;
pub mod virt_addr_from_indexes;

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // TODO: Blue screen with a frowny face and a QR Code
    log::error!("{}", info);
    hlt_loop()
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

// ↓ this replaces the `_start` function ↓
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let mut frame_buffer = boot_info.framebuffer.as_mut().unwrap();
    let mut display = Display::new(&mut frame_buffer);
    let data = include_bytes!("../rust-pride.tga");
    let image_size = 64;
    let tga: Tga<Rgb888> = Tga::from_slice(data).unwrap();

    // for pos_y in 0..display.framebuffer().info().height.div_ceil(image_size) {
    //     for pos_x in 0..display.framebuffer().info().width.div_ceil(image_size) {
    //         Image::new(
    //             &tga,
    //             Point::new((pos_x * image_size) as i32, (pos_y * image_size) as i32),
    //         )
    //         .draw(&mut display)
    //         .unwrap();
    //     }
    // }
    init_logger_with_framebuffer(&mut boot_info.framebuffer);
    init_gtd();
    init_interrupts();

    let phys_mem_offset_u64 = *boot_info
        .physical_memory_offset
        .as_ref()
        .expect("No physical memory mapped");
    let phys_mem_offset = VirtAddr::new(phys_mem_offset_u64);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(boot_info.memory_regions.deref_mut()) };

    allocator::init_heap(&mut mapper, &mut frame_allocator, phys_mem_offset)
        .expect("heap initialization failed");

    // allocate a number on the heap
    let heap_value = Box::new(41);
    log::info!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    log::info!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    log::info!(
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    );
    core::mem::drop(reference_counted);
    log::info!(
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    );

    init_apic(boot_info.rsdp_addr);

    // log::info!("Virtual address ranges: {:?}", ranges);
    // let total_virt_addresses_used: u64 = ranges.iter().map(|range| range.end - range.start).sum();
    // log::info!("Total virt addresses used: {}", total_virt_addresses_used);
    // log::info!("{:?}", boot_info.memory_regions.deref());

    log::info!("It did not crash");

    fn wait(x: usize) {
        for _ in 0..x {}
    }

    let mut i = 0;
    // loop {
    //     log::info!("Counter: {i}");
    //     i += 1;
    //     wait(1000000);
    // }

    hlt_loop();
}
