use common::syscall_take_frame_buffer::TakeFrameBufferOutputData;
use embedded_graphics::{
    image::Image,
    pixelcolor::Rgb888,
    prelude::{Drawable, OriginDimensions, Point},
};
use tinytga::Tga;

use crate::embedded_graphics_frame_buffer::FrameBufferDisplay;

pub fn draw_rust(frame_buffer: &mut TakeFrameBufferOutputData) {
    let mut display = FrameBufferDisplay::new(frame_buffer);
    let data = include_bytes!("../../rust-pride.tga");
    let image_size = 64;
    let tga: Tga<Rgb888> = Tga::from_slice(data).unwrap();

    for pos_y in 0..display.size().height.div_ceil(image_size) {
        for pos_x in 0..display.size().width.div_ceil(image_size) {
            Image::new(
                &tga,
                Point::new((pos_x * image_size) as i32, (pos_y * image_size) as i32),
            )
            .draw(&mut display)
            .unwrap();
        }
    }
}
