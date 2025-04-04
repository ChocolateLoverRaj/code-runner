use log::LevelFilter;

/// Maybe we will take config through a config file or options passed by the bootloader in the future
pub struct Config {
    pub kernel_log_com1: bool,
    pub kernel_log_screen: bool,
    pub kernel_log_spcr: bool,
    pub kernel_log_serial_colors: bool,
    pub kernel_log_serial_level_filter: LevelFilter,
}

pub const CONFIG: Config = Config {
    kernel_log_com1: true,
    kernel_log_screen: true,
    kernel_log_spcr: true,
    kernel_log_serial_colors: true,
    kernel_log_serial_level_filter: LevelFilter::Info,
};

/// The size in bytes of the `static` buffer used to store log messages before the global allocator is initialized.
pub const LOG_BUFFER_SIZE: usize = 0x100_000;
