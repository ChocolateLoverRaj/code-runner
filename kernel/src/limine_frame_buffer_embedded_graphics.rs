use core::{convert::Infallible, slice};

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, RgbColor, Size},
    primitives::Rectangle,
};
use limine::framebuffer::{Framebuffer, MemoryModel};
use thiserror::Error;

pub struct LimineFrameBufferEmbeddedGraphics<'a> {
    frame_buffer: Framebuffer<'a>,
}

impl LimineFrameBufferEmbeddedGraphics<'_> {
    fn buffer_mut(&mut self) -> &mut [u8] {
        let frame_buffer_len = (self.frame_buffer.pitch() * self.frame_buffer.height()) as usize;
        unsafe { slice::from_raw_parts_mut(self.frame_buffer.addr(), frame_buffer_len) }
    }

    pub fn shift_up(&mut self, amount: u32) {
        let pitch = self.frame_buffer.pitch();
        let buffer = self.buffer_mut();
        buffer.copy_within(amount as usize * pitch as usize..buffer.len(), 0);
    }
}

#[derive(Debug, Error)]
pub enum FromFrameBufferError {
    #[error("DrawTarget implemented for RGB888, but bpp doesn't match RGB888")]
    BppMismatch(u16),
    #[error("The frame buffer is not RGB")]
    NotRgb,
}

impl<'a> TryFrom<Framebuffer<'a>> for LimineFrameBufferEmbeddedGraphics<'a> {
    type Error = FromFrameBufferError;

    fn try_from(frame_buffer: Framebuffer<'a>) -> Result<Self, Self::Error> {
        if frame_buffer.bpp() != 8 * 4 {
            Err(FromFrameBufferError::BppMismatch(frame_buffer.bpp()))
        } else if frame_buffer.memory_model() != MemoryModel::RGB {
            Err(FromFrameBufferError::NotRgb)
        } else {
            Ok(Self { frame_buffer })
        }
    }
}

impl DrawTarget for LimineFrameBufferEmbeddedGraphics<'_> {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        pixels.into_iter().for_each(|pixel| {
            let point = pixel.0;
            if point.x < 0
                || point.y < 0
                || point.x as u64 >= self.frame_buffer.width()
                || point.y as u64 >= self.frame_buffer.height()
            {
                return;
            }
            let color = pixel.1;
            let mut n = 0;
            n |= ((color.r() as u32) & ((1 << self.frame_buffer.red_mask_size()) - 1))
                << self.frame_buffer.red_mask_shift();
            n |= ((color.g() as u32) & ((1 << self.frame_buffer.green_mask_size()) - 1))
                << self.frame_buffer.green_mask_shift();
            n |= ((color.b() as u32) & ((1 << self.frame_buffer.blue_mask_size()) - 1))
                << self.frame_buffer.blue_mask_shift();
            let bytes_per_pixel = (self.frame_buffer.bpp() / 8) as usize;
            let buffer_position = point.y as usize * self.frame_buffer.pitch() as usize
                + point.x as usize * bytes_per_pixel;
            let buffer = self.buffer_mut();
            buffer[buffer_position..buffer_position + bytes_per_pixel as usize]
                .copy_from_slice(&n.to_ne_bytes());
        });
        Ok(())
    }
}

impl Dimensions for LimineFrameBufferEmbeddedGraphics<'_> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        Rectangle {
            top_left: Point { x: 0, y: 0 },
            size: Size {
                width: self.frame_buffer.width().try_into().unwrap(),
                height: self.frame_buffer.height().try_into().unwrap(),
            },
        }
    }
}
