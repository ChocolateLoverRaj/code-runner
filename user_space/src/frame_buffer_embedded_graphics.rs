use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, RgbColor, Size},
    primitives::Rectangle,
};
use thiserror::Error;

use crate::screen::Screen;

pub struct FrameBufferEmbeddedGraphics<'a> {
    screen: &'a mut Screen,
}

#[derive(Debug, Error)]
pub enum NewError {
    #[error("DrawTarget implemented for RGB888, but bpp doesn't match RGB888")]
    BppMismatch(u16),
}
impl<'a> FrameBufferEmbeddedGraphics<'a> {
    pub fn new(screen: &'a mut Screen) -> Result<Self, NewError> {
        let bits_per_pixel = screen.info().bits_per_pixel;
        if bits_per_pixel != 8 * 4 {
            Err(NewError::BppMismatch(bits_per_pixel))
        } else {
            Ok(Self { screen })
        }
    }

    fn tranform_pixel(&self, color: Rgb888) -> [u8; 4] {
        let mut n = 0;
        n |= ((color.r() as u32) & ((1 << self.screen.info().red_mask_size) - 1))
            << self.screen.info().red_mask_shift;
        n |= ((color.g() as u32) & ((1 << self.screen.info().green_mask_size) - 1))
            << self.screen.info().green_mask_shift;
        n |= ((color.b() as u32) & ((1 << self.screen.info().blue_mask_size) - 1))
            << self.screen.info().blue_mask_shift;
        n.to_ne_bytes()
    }
}

impl DrawTarget for FrameBufferEmbeddedGraphics<'_> {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let bytes_per_pixel = (self.screen.info().bits_per_pixel / 8) as usize;
        pixels.into_iter().for_each(|pixel| {
            let point = pixel.0;
            if point.x < 0
                || point.y < 0
                || point.x as u64 >= self.screen.info().width
                || point.y as u64 >= self.screen.info().height
            {
                return;
            }
            let color = pixel.1;
            let buffer_position = point.y as usize * self.screen.info().pitch as usize
                + point.x as usize * bytes_per_pixel;
            let pixel = self.tranform_pixel(color);
            let buffer = self.screen.frame_buffer_mut();
            buffer[buffer_position..buffer_position + bytes_per_pixel as usize]
                .copy_from_slice(&pixel);
        });
        Ok(())
    }

    // The advantage of implementing this method is that we can just compute the pixel and then copy it
    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let pixel = self.tranform_pixel(color);
        let info = *self.screen.info();
        let buffer = self.screen.frame_buffer_mut();
        let bytes_per_pixel = (info.bits_per_pixel / 8) as usize;
        for x in area.top_left.x..area.top_left.x + area.size.width as i32 {
            let buffer_position =
                area.top_left.y as usize * info.pitch as usize + x as usize * bytes_per_pixel;
            buffer[buffer_position..buffer_position + bytes_per_pixel as usize]
                .copy_from_slice(&pixel);
        }
        let top_row_start = area.top_left.y as usize * info.pitch as usize
            + area.top_left.x as usize * bytes_per_pixel;
        let top_row = top_row_start..top_row_start + area.size.width as usize * bytes_per_pixel;
        for y in area.top_left.y..area.top_left.y + area.size.height as i32 {
            let row_start =
                y as usize * info.pitch as usize + area.top_left.x as usize * bytes_per_pixel;
            buffer.copy_within(top_row.clone(), row_start);
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let pixel = self.tranform_pixel(color);
        for pixel_index in 0..(self.screen.info().width * self.screen.info().height) as usize {
            self.screen.frame_buffer_mut()[pixel_index * 4..pixel_index * 4 + 4]
                .copy_from_slice(&pixel);
        }
        Ok(())
    }
}

impl Dimensions for FrameBufferEmbeddedGraphics<'_> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        Rectangle {
            top_left: Point { x: 0, y: 0 },
            size: Size {
                width: self.screen.info().width.try_into().unwrap(),
                height: self.screen.info().height.try_into().unwrap(),
            },
        }
    }
}
