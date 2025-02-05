use bootloader_x86_64_common::serial::SerialPort;
use core::fmt::Write;
use log::Log;

pub struct SerialLogger {
    serial_port: spin::Mutex<SerialPort>,
}

impl SerialLogger {
    /// # Safety
    /// Initializes a `SerialPort`
    pub unsafe fn init() -> Self {
        Self {
            serial_port: spin::Mutex::new(unsafe { SerialPort::init() }),
        }
    }
}

impl Log for SerialLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut serial_port = self.serial_port.lock();
        writeln!(serial_port, "{:5}: {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}
