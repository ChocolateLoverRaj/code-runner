use core::fmt::Write;
use log::Log;
use spinning_top::Spinlock;

pub struct LockedWriteLogger<T> {
    writer: Spinlock<T>,
}

impl<T> LockedWriteLogger<T> {
    pub const fn new(writer: T) -> Self {
        Self {
            writer: Spinlock::new(writer),
        }
    }
}

impl<T: Send + Write> Log for LockedWriteLogger<T> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut writer = self.writer.lock();
        writeln!(writer, "{:5}: {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}
