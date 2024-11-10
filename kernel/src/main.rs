#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

use apic::init_apic;
use bootloader_api::BootInfo;
use embedded_graphics::{
    image::Image,
    pixelcolor::Rgb888,
    prelude::{Drawable, Point, Primitive, Size, Transform},
    primitives::{PrimitiveStyle, Rectangle},
};
use framebuffer::Display;
use gtd::init_gtd;
use hlt_loop::hlt_loop;
use interrupts::init_interrupts;
use logger::init_logger_with_framebuffer;
use tinytga::Tga;

pub mod apic;
pub mod framebuffer;
pub mod gtd;
pub mod hlt_loop;
pub mod interrupts;
pub mod logger;

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // TODO: Blue screen with a frowny face and a QR Code
    log::error!("{}", info);
    hlt_loop()
}

bootloader_api::entry_point!(kernel_main);

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
    init_apic(boot_info.rsdp_addr);

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
