use log::Log;

pub struct CombinedLogger<'a, const N: usize> {
    pub loggers: &'a [&'a dyn Log],
}

impl<const N: usize> Log for CombinedLogger<'_, N> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.loggers.iter().any(|logger| logger.enabled(metadata))
    }

    fn log(&self, record: &log::Record) {
        for logger in self.loggers {
            logger.log(record);
        }
    }

    fn flush(&self) {
        for logger in self.loggers {
            logger.flush();
        }
    }
}
