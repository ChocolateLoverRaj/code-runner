#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

// #[allow(unused)]
// #[macro_use]
// extern crate alloc;

use core::{ops::DerefMut, panic::PanicInfo};

use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use gtd::init_gtd;
use hlt_loop::hlt_loop;
use interrupts::init_interrupts;
use logger::init_logger_with_framebuffer;
use x86_64::VirtAddr;

pub mod allocator;
// pub mod apic;
pub mod colorful_logger;
pub mod combined_logger;
pub mod draw_rust;
pub mod embedded_graphics_writer;
pub mod find_used_virt_addrs;
pub mod frame_buffer;
pub mod get_rgb_color;
pub mod gtd;
pub mod hlt_loop;
pub mod interrupts;
pub mod logger;
pub mod logger_without_interrupts;
pub mod memory;
pub mod serial_logger;
pub mod set_color;
pub mod split_draw_target;
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
    // let frame_buffer = boot_info.framebuffer.as_mut().unwrap();
    // draw_rust::draw_rust(frame_buffer);
    init_gtd();
    init_interrupts();
    init_logger_with_framebuffer(&mut boot_info.framebuffer);
    let phys_mem_offset = VirtAddr::new(
        *boot_info
            .physical_memory_offset
            .as_ref()
            .expect("No physical memory mapped"),
    );
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(boot_info.memory_regions.deref_mut()) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    log::info!("It did not crash");

    hlt_loop();
}
