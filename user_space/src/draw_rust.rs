use core::fmt::Debug;

use embedded_graphics::{
    image::Image,
    pixelcolor::{Gray8, Rgb555, Rgb888},
    prelude::{DrawTarget, Drawable, OriginDimensions, PixelColor, Point},
};
use tinytga::Tga;

pub fn draw_rust<D: DrawTarget + OriginDimensions>(display: &mut D)
where
    D::Error: Debug,
    D::Color: PixelColor + From<Gray8> + From<Rgb555> + From<Rgb888>,
{
    let data = include_bytes!("../../rust-pride.tga");
    let image_size = 64;
    let tga: Tga<D::Color> = Tga::from_slice(data).unwrap();

    for pos_y in 0..display.size().height.div_ceil(image_size) {
        for pos_x in 0..display.size().width.div_ceil(image_size) {
            Image::new(
                &tga,
                Point::new((pos_x * image_size) as i32, (pos_y * image_size) as i32),
            )
            .draw(display)
            .unwrap();
        }
    }
}
