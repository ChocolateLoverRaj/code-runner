use bootloader_api::info::{FrameBuffer, PixelFormat};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{Rgb888, RgbColor},
    prelude::{Dimensions, Point},
    Pixel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

pub fn set_pixel_in(framebuffer: &mut FrameBuffer, position: Position, color: Rgb888) {
    let info = framebuffer.info();

    // calculate offset to first byte of pixel
    let byte_offset = {
        // use stride to calculate pixel offset of target line
        let line_offset = position.y * info.stride;
        // add x position to get the absolute pixel offset in buffer
        let pixel_offset = line_offset + position.x;
        // convert to byte offset
        pixel_offset * info.bytes_per_pixel
    };

    // set pixel based on color format
    let pixel_buffer = &mut framebuffer.buffer_mut()[byte_offset..];
    match info.pixel_format {
        PixelFormat::Rgb => {
            pixel_buffer[0] = color.r();
            pixel_buffer[1] = color.g();
            pixel_buffer[2] = color.b();
        }
        PixelFormat::Bgr => {
            pixel_buffer[0] = color.b();
            pixel_buffer[1] = color.g();
            pixel_buffer[2] = color.r();
        }
        PixelFormat::U8 => {
            // use a simple average-based grayscale transform
            let gray = color.r() / 3 + color.g() / 3 + color.b() / 3;
            pixel_buffer[0] = gray;
        }
        other => panic!("unknown pixel format {other:?}"),
    }
}

pub struct Display<'f> {
    framebuffer: &'f mut FrameBuffer,
}

impl<'f> Display<'f> {
    pub fn new(framebuffer: &mut FrameBuffer) -> Display {
        Display { framebuffer }
    }

    fn draw_pixel(&mut self, Pixel(Point { x, y }, color): Pixel<Rgb888>) {
        set_pixel_in(
            self.framebuffer,
            Position {
                x: x as usize,
                y: y as usize,
            },
            color,
        );
    }

    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }
}

impl<'f> DrawTarget for Display<'f> {
    type Color = Rgb888;

    /// Drawing operations can never fail.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels.into_iter() {
            self.draw_pixel(pixel);
        }

        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let info = self.framebuffer.info();
        let buffer = self.framebuffer.buffer_mut();
        let draw_pixel = |pixel_buffer: &mut [u8], color: Self::Color| {
            match info.pixel_format {
                PixelFormat::Rgb => {
                    pixel_buffer[0] = color.r();
                    pixel_buffer[1] = color.g();
                    pixel_buffer[2] = color.b();
                }
                PixelFormat::Bgr => {
                    pixel_buffer[0] = color.b();
                    pixel_buffer[1] = color.g();
                    pixel_buffer[2] = color.r();
                }
                PixelFormat::U8 => {
                    // use a simple average-based grayscale transform
                    let gray = color.r() / 3 + color.g() / 3 + color.b() / 3;
                    pixel_buffer[0] = gray;
                }
                other => panic!("unknown pixel format {other:?}"),
            }
        };
        let mut colors = colors.into_iter();
        for y in area.top_left.y as usize
            ..(area.top_left.y as usize + area.size.height as usize).min(info.height)
        {
            for x in area.top_left.x as usize
                ..(area.top_left.x as usize + area.size.width as usize).min(info.width)
            {
                let start = y * info.stride + x;
                let pixel_buffer =
                    &mut buffer[start * info.bytes_per_pixel..(start + 1) * info.bytes_per_pixel];
                draw_pixel(pixel_buffer, colors.next().unwrap());
            }
        }
        Ok(())
    }

    fn fill_solid(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        let info = self.framebuffer.info();

        match info.pixel_format {
            PixelFormat::Bgr => {
                let top_left_pixel_index = {
                    // use stride to calculate pixel offset of target line
                    let line_offset = area.top_left.y as usize * info.stride as usize;
                    // add x position to get the absolute pixel offset in buffer
                    let pixel_offset = line_offset + area.top_left.x as usize;
                    // convert to byte offset
                    pixel_offset * info.bytes_per_pixel
                };
                let buffer = self.framebuffer.buffer_mut();
                let top_left_pixel =
                    &mut buffer[top_left_pixel_index..top_left_pixel_index + info.bytes_per_pixel];
                top_left_pixel[0] = color.g();
                top_left_pixel[1] = color.b();
                top_left_pixel[2] = color.r();
                // For testing
                // top_left_pixel[0] = 50;
                // top_left_pixel[1] = 50;
                // top_left_pixel[2] = 50;
                for x in
                    area.top_left.x as usize..area.top_left.x as usize + area.size.width as usize
                {
                    let pixel_index =
                        (area.top_left.y as usize * info.stride + x) * info.bytes_per_pixel;
                    buffer.copy_within(
                        top_left_pixel_index..top_left_pixel_index + info.bytes_per_pixel,
                        pixel_index,
                    );
                }
                for y in
                    area.top_left.y as usize..area.top_left.y as usize + area.size.height as usize
                {
                    let start_index = (y as usize * info.stride + area.top_left.x as usize)
                        * info.bytes_per_pixel;
                    buffer.copy_within(
                        top_left_pixel_index
                            ..top_left_pixel_index
                                + info.bytes_per_pixel * area.size.width as usize,
                        start_index,
                    );
                }
            }
            other => panic!("unknown pixel format {other:?}"),
        };
        // log::info!("Clear called");
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        if color == Rgb888::BLACK {
            self.framebuffer.buffer_mut().fill(0);
        } else {
            self.fill_solid(&self.bounding_box(), color)?;
        }
        Ok(())
    }
}

impl<'f> OriginDimensions for Display<'f> {
    fn size(&self) -> Size {
        let info = self.framebuffer.info();

        Size::new(info.width as u32, info.height as u32)
    }
}
