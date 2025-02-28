use core::ops::Deref;

use alloc::boxed::Box;
use log::Log;
use spinning_top::Spinlock;

pub enum DynamicLogger<'a> {
    Static(&'a dyn Log),
    Heap(Box<dyn Log>),
}

impl<'a> Deref for DynamicLogger<'a> {
    type Target = dyn Log + 'a;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Static(logger) => logger,
            Self::Heap(logger) => logger.deref(),
        }
    }
}

pub struct DynamicCombinedLogger<'a, const N: usize> {
    pub loggers: spin::Mutex<heapless::Vec<DynamicLogger<'a>, N>>,
}

impl<const N: usize> Log for DynamicCombinedLogger<'_, N> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.loggers
            .lock()
            .iter()
            .any(|logger| logger.enabled(metadata))
    }

    fn log(&self, record: &log::Record) {
        for logger in self.loggers.lock().iter() {
            logger.log(record);
        }
    }

    fn flush(&self) {
        for logger in self.loggers.lock().iter() {
            logger.flush();
        }
    }
}
