use embedded_graphics::mono_font::MonoFont;
use log::LevelFilter;

pub struct LogSerialConfig {
    /// If `true`, the kernel will log some stuff before it checks for SPCR.
    /// If SPCR is present, the kernel will not log to COM1 and will log to SPCR (regardless of value).
    pub always_log_com1: bool,
    pub colors: bool,
    pub level_filter: LevelFilter,
}

pub struct LogScreenConfig {
    pub font: MonoFont<'static>,
    pub level_filter: LevelFilter,
}

/// Maybe we will take config through a config file or options passed by the bootloader in the future
pub struct Config {
    pub kernel_log_serial: Option<LogSerialConfig>,
    pub kernel_log_screen: Option<LogScreenConfig>,
    pub kernel_log_sample_messages: bool,
}

pub const CONFIG: Config = Config {
    kernel_log_serial: Some(LogSerialConfig {
        always_log_com1: true,
        colors: true,
        level_filter: LevelFilter::Debug,
    }),
    kernel_log_screen: Some(LogScreenConfig {
        font: embedded_graphics::mono_font::iso_8859_16::FONT_10X20,
        level_filter: LevelFilter::Warn,
    }),
    // kernel_log_screen: None,
    kernel_log_sample_messages: true,
};

/// The size in bytes of the `static` buffer used to store log messages before the global allocator is initialized.
pub const LOG_BUFFER_SIZE: usize = 0x100_000;
/// 16 MiB
pub const GLOBAL_ALLOCATOR_SIZE: usize = 16 * 0x400 * 0x400;
/// 64 KiB
pub const SYSCALL_HANDLER_STACK_SIZE: usize = 64 * 0x400;
