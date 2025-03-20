use core::fmt::{Display, Write};
use log::{Level, Log};
use owo_colors::{AnsiColors, Color, OwoColorize};
use spinning_top::Spinlock;

pub struct LockedWriteLogger<T> {
    writer: Spinlock<T>,
    colors_enabled: bool,
}

impl<T> LockedWriteLogger<T> {
    pub const fn new(writer: T, colors_enabled: bool) -> Self {
        Self {
            writer: Spinlock::new(writer),
            colors_enabled,
        }
    }
}

impl<T: Send + Write> Log for LockedWriteLogger<T> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut writer = self.writer.lock();
        let level = record.level();
        let level: &dyn Display = if self.colors_enabled {
            &level.color(match level {
                Level::Error => AnsiColors::Red,
                Level::Warn => AnsiColors::Yellow,
                Level::Info => AnsiColors::Blue,
                Level::Debug => AnsiColors::Green,
                Level::Trace => AnsiColors::Cyan,
            })
        } else {
            &level
        };
        writeln!(writer, "{:5}: {}", level, record.args()).unwrap();
    }

    fn flush(&self) {}
}
