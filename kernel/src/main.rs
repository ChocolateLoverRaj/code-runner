#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[allow(unused)]
#[macro_use]
extern crate alloc;

use core::{ops::DerefMut, panic::PanicInfo};

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use gtd::init_gtd;
use hlt_loop::hlt_loop;
use interrupts::init_interrupts;
use logger::init_logger_with_framebuffer;
use x86_64::VirtAddr;

pub mod allocator;
pub mod apic;
pub mod draw_rust;
pub mod find_used_virt_addrs;
pub mod frame_buffer;
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
    let frame_buffer = boot_info.framebuffer.as_mut().unwrap();

    // draw_rust(frame_buffer);
    init_logger_with_framebuffer(&mut boot_info.framebuffer);
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
    init_gtd();
    init_interrupts();

    log::info!("It did not crash");

    hlt_loop();
}
