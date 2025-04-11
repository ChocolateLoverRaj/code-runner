use embedded_graphics::mono_font::{self, MonoFont};
use log::LevelFilter;

/// Maybe we will take config through a config file or options passed by the bootloader in the future
pub struct Config {
    /// If `true`, the kernel will log to COM1. If SPCR is present, the kernel will stop logging to COM1
    pub kernel_log_com1: bool,
    /// If `true`, the kernel will log to the screen, even if it is also logging to serial
    pub kernel_log_screen: Option<MonoFont<'static>>,
    /// If `true`, the kernel will log to SPCR and stop logging to COM1 if it was before. If `false` and SPCR is present, the kernel will not log to COM1 or SPCR.
    pub kernel_log_spcr: bool,
    pub kernel_log_serial_colors: bool,
    pub kernel_log_serial_level_filter: LevelFilter,
}

pub const CONFIG: Config = Config {
    kernel_log_com1: true,
    kernel_log_screen: Some(mono_font::iso_8859_16::FONT_10X20),
    kernel_log_spcr: true,
    kernel_log_serial_colors: true,
    kernel_log_serial_level_filter: LevelFilter::Info,
};

/// The size in bytes of the `static` buffer used to store log messages before the global allocator is initialized.
pub const LOG_BUFFER_SIZE: usize = 0x100_000;
