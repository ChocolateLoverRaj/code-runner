use bootloader_api::{info::FrameBuffer, info::Optional};
use conquer_once::spin::OnceCell;
use embedded_graphics::{
    mono_font::iso_8859_16::FONT_10X20, pixelcolor::Rgb888, prelude::RgbColor,
};

use crate::{
    colorful_logger::ColorfulLogger, combined_logger::CombinedLogger,
    embedded_graphics_writer::EmbeddedGraphicsWriter, frame_buffer::Display,
    get_rgb_color::GET_RGB_COLOR, logger_without_interrupts::LockedLoggerWithoutInterrupts,
    serial_logger::SerialLogger,
};

static SCREEN_LOGGER: OnceCell<ColorfulLogger<Rgb888, EmbeddedGraphicsWriter<Display>>> =
    OnceCell::uninit();
static SERIAL_LOGGER: OnceCell<SerialLogger> = OnceCell::uninit();
static LOGGER: OnceCell<LockedLoggerWithoutInterrupts<CombinedLogger<'static, 2>>> =
    OnceCell::uninit();

pub fn init_logger_with_framebuffer(frame_buffer_optional: &'static mut Optional<FrameBuffer>) {
    let logger = LOGGER.get_or_init(move || {
        LockedLoggerWithoutInterrupts::new({
            let screen_logger = SCREEN_LOGGER.get_or_init(|| {
                ColorfulLogger::new(
                    EmbeddedGraphicsWriter::new(
                        Display::new(frame_buffer_optional.as_mut().unwrap()),
                        FONT_10X20,
                        Rgb888::BLACK,
                    ),
                    GET_RGB_COLOR,
                )
            });
            let serial_logger = SERIAL_LOGGER.get_or_init(|| unsafe { SerialLogger::init() });
            CombinedLogger {
                loggers: [screen_logger, serial_logger],
            }
        })
    });
    log::set_logger(logger).expect("Logger already set");
    log::set_max_level(log::LevelFilter::Info);
    log::info!("Logger initialized");
}
