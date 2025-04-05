use log::Log;
use x86_64::instructions::interrupts::without_interrupts;

#[derive(Debug)]
pub struct LoggerWithoutInterrupts<T: Log> {
    pub logger: T,
}

impl<T: Log> LoggerWithoutInterrupts<T> {
    pub const fn new(logger: T) -> Self {
        Self { logger }
    }
}

impl<T: Log> Log for LoggerWithoutInterrupts<T> {
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
