use log::Log;
use x86_64::instructions::interrupts::without_interrupts;

pub struct LockedLoggerWithoutInterrupts<T: Log> {
    logger: T,
}

impl<T: Log> LockedLoggerWithoutInterrupts<T> {
    pub fn new(logger: T) -> Self {
        Self { logger }
    }
}

impl<T: Log> Log for LockedLoggerWithoutInterrupts<T> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        without_interrupts(|| self.logger.enabled(metadata))
    }

    fn log(&self, record: &log::Record) {
        without_interrupts(|| self.logger.log(record))
    }

    fn flush(&self) {
        without_interrupts(|| self.logger.flush())
    }
}
