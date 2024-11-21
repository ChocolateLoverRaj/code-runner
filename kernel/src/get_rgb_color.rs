use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
use log::Level;

use crate::colorful_logger::GetColor;

pub const GET_RGB_COLOR: GetColor<Rgb888> = |level| match level {
    Level::Error => Rgb888::RED,
    Level::Warn => Rgb888::YELLOW,
    Level::Info => Rgb888::BLUE,
    Level::Debug => Rgb888::GREEN,
    Level::Trace => Rgb888::WHITE,
};
