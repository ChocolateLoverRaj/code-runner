use core::{
    alloc::{AllocError, Allocator},
    fmt::Write,
    mem::MaybeUninit,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::vec::Vec;
use linked_list_allocator::LockedHeap;
use log::{LevelFilter, Log};
use spinning_top::Spinlock;
use uart_16550::{port::PortAccessedRegister, uart_16550::Uart16550Registers};
use util::init_later::InitLater;

use crate::{
    config::{CONFIG, LOG_BUFFER_SIZE},
    logger_without_interrupts::LoggerWithoutInterrupts,
    write_logger::LockedWriteLogger,
};

pub static COM1_IN_USE: AtomicBool = AtomicBool::new(false);

struct Logger {
    serial_logger: Option<LockedWriteLogger<Uart16550Registers<PortAccessedRegister>>>,
}

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        todo!()
    }

    fn log(&self, record: &log::Record) {
        if let Some(serial_logger) = &self.serial_logger {
            if record.level() <= CONFIG.kernel_log_serial_level_filter {
                serial_logger.log(record);
            }
        }
        LOG_MESSAGES
            .lock()
            .push(LogMessage::Kernel(KernelLogMessage {
                cpu: 0,
                message: LogMessageWithLevel {
                    level: record.level(),
                    message: {
                        let mut message = string_alloc::String::new_in(&LOG_MESSAGES_ALLOCATOR);
                        write!(message, "{}", record.args()).unwrap();
                        message
                    },
                },
            }));
    }

    fn flush(&self) {
        todo!()
    }
}

impl Logger {
    pub fn new() -> Self {
        Self {
            serial_logger: if CONFIG.kernel_log_com1 {
                if let Ok(_) =
                    COM1_IN_USE.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                {
                    Some(LockedWriteLogger::new(
                        unsafe { uart_16550::port::new(0x3F8) },
                        CONFIG.kernel_log_serial_colors,
                    ))
                } else {
                    None
                }
            } else {
                None
            },
        }
    }
}

pub struct LogMessageWithLevel<A: Allocator + Clone + Default> {
    pub level: log::Level,
    pub message: string_alloc::String<A>,
}

pub struct KernelLogMessage<A: Allocator + Clone + Default> {
    /// The CPU ID, based on what the bootloader provides
    pub cpu: usize,
    pub message: LogMessageWithLevel<A>,
}

pub struct UserProcessLogMessage<A: Allocator + Clone + Default> {
    pub process_id: usize,
    pub message: LogMessageWithLevel<A>,
}

pub enum LogMessage<A: Allocator + Clone + Default> {
    Kernel(KernelLogMessage<A>),
    UserProcess(UserProcessLogMessage<A>),
}

static LOGGER: InitLater<LoggerWithoutInterrupts<Logger>> = InitLater::uninit();

static mut LOGGER_BYTES: [MaybeUninit<u8>; LOG_BUFFER_SIZE] =
    [MaybeUninit::uninit(); LOG_BUFFER_SIZE];
pub static LOG_MESSAGES_ALLOCATOR: StaticLinkedListAllocator = StaticLinkedListAllocator {
    heap: LockedHeap::empty(),
};
pub static LOG_MESSAGES: Spinlock<
    Vec<LogMessage<&StaticLinkedListAllocator>, &StaticLinkedListAllocator>,
> = Spinlock::new(Vec::new_in(&LOG_MESSAGES_ALLOCATOR));

pub struct StaticLinkedListAllocator {
    pub heap: LockedHeap,
}

impl Clone for StaticLinkedListAllocator {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

impl Default for StaticLinkedListAllocator {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Default for &StaticLinkedListAllocator {
    fn default() -> Self {
        unimplemented!()
    }
}

unsafe impl Allocator for StaticLinkedListAllocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let start = self
            .heap
            .lock()
            .allocate_first_fit(layout)
            .map_err(|_| AllocError)?;
        Ok(NonNull::new(core::ptr::slice_from_raw_parts_mut(
            start.as_ptr(),
            layout.size(),
        ))
        .unwrap())
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
        unsafe { self.heap.lock().deallocate(ptr, layout) };
    }
}

pub fn init() {
    LOG_MESSAGES_ALLOCATOR
        .heap
        .lock()
        .init_from_slice(unsafe { &mut LOGGER_BYTES });
    log::set_logger(
        LOGGER
            .try_init(LoggerWithoutInterrupts::new(Logger::new()))
            .unwrap(),
    )
    .unwrap();
    log::set_max_level(LevelFilter::Trace);
}
