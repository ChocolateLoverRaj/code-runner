use core::{
    alloc::{AllocError, Allocator},
    cell::SyncUnsafeCell,
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
        let mut messages = LOG_MESSAGES.lock();
        let alloc = *messages.allocator();
        messages.push(LogMessage::Kernel(KernelLogMessage {
            cpu: 0,
            message: LogMessageWithLevel {
                level: record.level(),
                message: {
                    let mut message = string_alloc::String::new_in(alloc);
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

pub static LOG_MESSAGES_ALLOCATOR: StaticLinkedListAllocator<LOG_BUFFER_SIZE> =
    StaticLinkedListAllocator::uninit();
pub static LOG_MESSAGES: Spinlock<
    Vec<
        LogMessage<&StaticLinkedListAllocator<LOG_BUFFER_SIZE>>,
        &StaticLinkedListAllocator<LOG_BUFFER_SIZE>,
    >,
> = Spinlock::new(Vec::new_in(&LOG_MESSAGES_ALLOCATOR));

/// N must be a multiple of 0x1000
#[repr(C, align(0x1000))]
pub struct PreReservedPages<const N: usize> {
    pub bytes: [MaybeUninit<u8>; N],
}

/// N must be a multiple of 0x1000
pub struct StaticLinkedListAllocator<const N: usize> {
    pub pre_reserved_pages: SyncUnsafeCell<PreReservedPages<N>>,
    pub heap: LockedHeap,
}

impl<const N: usize> StaticLinkedListAllocator<N> {
    pub const fn uninit() -> Self {
        Self {
            pre_reserved_pages: SyncUnsafeCell::new(PreReservedPages {
                bytes: [MaybeUninit::uninit(); N],
            }),
            heap: LockedHeap::empty(),
        }
    }

    /// # Safety
    /// This function must be called exactly once.
    pub unsafe fn init(&'static self) {
        unsafe {
            self.heap
                .lock()
                .init(self.pre_reserved_pages.get().cast(), N)
        };
    }
}

impl<const N: usize> Clone for StaticLinkedListAllocator<N> {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

// impl Default for StaticLinkedListAllocator {
//     fn default() -> Self {
//         unimplemented!()
//     }
// }

impl<const N: usize> Default for &StaticLinkedListAllocator<N> {
    fn default() -> Self {
        unimplemented!()
    }
}

unsafe impl<const N: usize> Allocator for StaticLinkedListAllocator<N> {
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
    unsafe { LOG_MESSAGES_ALLOCATOR.init() };
    log::set_logger(
        LOGGER
            .try_init(LoggerWithoutInterrupts::new(Logger::new()))
            .unwrap(),
    )
    .unwrap();
    log::set_max_level(LevelFilter::Trace);
}
