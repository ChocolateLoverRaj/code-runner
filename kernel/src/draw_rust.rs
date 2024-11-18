use bootloader_api::info::FrameBuffer;
use embedded_graphics::{
    image::Image,
    pixelcolor::Rgb888,
    prelude::{Drawable, Point},
};
use tinytga::Tga;

use crate::frame_buffer::Display;

pub fn draw_rust(frame_buffer: &mut FrameBuffer) {
    let mut display = Display::new(frame_buffer);
    let data = include_bytes!("../rust-pride.tga");
    let image_size = 64;
    let tga: Tga<Rgb888> = Tga::from_slice(data).unwrap();

    for pos_y in 0..display.framebuffer().info().height.div_ceil(image_size) {
        for pos_x in 0..display.framebuffer().info().width.div_ceil(image_size) {
            Image::new(
                &tga,
                Point::new((pos_x * image_size) as i32, (pos_y * image_size) as i32),
            )
            .draw(&mut display)
            .unwrap();
        }
    }
}
