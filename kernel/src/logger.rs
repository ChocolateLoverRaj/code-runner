use bootloader_api::info::FrameBuffer;
use conquer_once::spin::OnceCell;
use embedded_graphics::{
    mono_font::iso_8859_16::FONT_10X20, pixelcolor::Rgb888, prelude::RgbColor,
};
use log::Log;
use spinning_top::Spinlock;
use uart_16550_2::{port::PortAccessedRegister, uart_16550::Uart16550Registers};

use crate::{
    colorful_logger::ColorfulLogger,
    dynamic_combined_logger::{DynamicCombinedLogger, DynamicLogger},
    embedded_graphics_writer::EmbeddedGraphicsWriter,
    frame_buffer::Display,
    get_rgb_color::GET_RGB_COLOR,
    logger_without_interrupts::LockedLoggerWithoutInterrupts,
    write_logger::LockedWriteLogger,
};

static SCREEN_LOGGER: OnceCell<ColorfulLogger<Rgb888, EmbeddedGraphicsWriter<Display>>> =
    OnceCell::uninit();
static SERIAL_LOGGER: LockedWriteLogger<Uart16550Registers<PortAccessedRegister>> =
    LockedWriteLogger::new(unsafe { uart_16550_2::port::new(0x3F8) });
static LOGGERS: OnceCell<heapless::Vec<&'static dyn Log, 2>> = OnceCell::uninit();
static LOGGER: LockedLoggerWithoutInterrupts<DynamicCombinedLogger<'static, 2>> =
    LockedLoggerWithoutInterrupts::new(DynamicCombinedLogger {
        loggers: spin::Mutex::new(heapless::Vec::new()),
    });

/// This function should only be called once.
pub fn init_logger_with_framebuffer(frame_buffer: Option<&'static mut FrameBuffer>) {
    let mut loggers = LOGGER.logger.loggers.lock();
    loggers
        .push(DynamicLogger::Static(&SERIAL_LOGGER))
        .map_err(|_| ())
        .unwrap();
    if let Some(frame_buffer) = frame_buffer {
        loggers
            .push({
                DynamicLogger::Static(SCREEN_LOGGER.get_or_init(|| {
                    ColorfulLogger::new(
                        EmbeddedGraphicsWriter::new(
                            Display::new(frame_buffer),
                            FONT_10X20,
                            Rgb888::BLACK,
                        ),
                        GET_RGB_COLOR,
                    )
                }))
            })
            .map_err(|_| ())
            .unwrap();
    }
    log::set_logger(&LOGGER).expect("Logger already set");
    log::set_max_level(log::LevelFilter::Info);
    log::debug!("Logger initialized");
}

/// This function should be called after the logger is initialized. Interrupts must be disabled while calling this function.
pub fn replace_serial_logger(new_serial_logger: DynamicLogger<'static>) {
    LOGGER.logger.loggers.lock()[0] = new_serial_logger;
}
