use core::{
    alloc::{AllocError, Allocator},
    cell::SyncUnsafeCell,
    fmt::Write,
    mem::MaybeUninit,
    ops::DerefMut,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::{alloc::Global, vec::Vec};
use linked_list_allocator::LockedHeap;
use log::{LevelFilter, Log};
use spinning_top::Spinlock;
use uart_16550::{port::PortAccessedRegister, uart_16550::Uart16550Registers};
use util::init_later::InitLater;

use crate::{
    config::{CONFIG, LOG_BUFFER_SIZE},
    logger_without_interrupts::LoggerWithoutInterrupts,
    pt_allocator_2::pre_reserved_pages::PreReservedPages,
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
        if let Ok(_) = TEMPORARILY_DISABLE_LOGGING.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            let mut messages = LOG_MESSAGES.lock();
            match messages.deref_mut() {
                LogMessages::InitialAllocator(messages) => {
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
                LogMessages::GlobalAllocator(messages) => {
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
            }
            drop(messages);
            TEMPORARILY_DISABLE_LOGGING.store(false, Ordering::Release);
        }
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

impl<A: Allocator + Clone + Default> LogMessage<A> {
    pub fn clone_in<
        NewA: Allocator + Clone + Default,
        F: FnOnce(&LogMessageWithLevel<A>) -> LogMessageWithLevel<NewA>,
    >(
        &self,
        f: F,
    ) -> LogMessage<NewA> {
        match self {
            LogMessage::Kernel(message) => LogMessage::Kernel(KernelLogMessage {
                cpu: message.cpu,
                message: f(&message.message),
            }),
            LogMessage::UserProcess(message) => LogMessage::UserProcess(UserProcessLogMessage {
                process_id: message.process_id,
                message: f(&message.message),
            }),
        }
    }
}

static LOGGER: InitLater<LoggerWithoutInterrupts<Logger>> = InitLater::uninit();

pub static LOG_MESSAGES_ALLOCATOR: StaticLinkedListAllocator<LOG_BUFFER_SIZE> =
    StaticLinkedListAllocator::uninit();

pub enum LogMessages<A: Allocator + Clone + Default> {
    InitialAllocator(Vec<LogMessage<A>, A>),
    GlobalAllocator(Vec<LogMessage<Global>, Global>),
}

impl<A: Allocator + Clone + Default> LogMessages<A> {
    pub fn clone_in_global(&self) -> Self {
        match self {
            Self::InitialAllocator(messages) => Self::GlobalAllocator(
                messages
                    .into_iter()
                    .map(|message| {
                        message.clone_in(|message| LogMessageWithLevel {
                            level: message.level,
                            message: string_alloc::String::from_str_in(&message.message, Global),
                        })
                    })
                    .collect(),
            ),
            _ => panic!("Already backed by global allocator"),
        }
    }
}

pub static LOG_MESSAGES: Spinlock<LogMessages<&StaticLinkedListAllocator<LOG_BUFFER_SIZE>>> =
    Spinlock::new(LogMessages::InitialAllocator(Vec::new_in(
        &LOG_MESSAGES_ALLOCATOR,
    )));
static TEMPORARILY_DISABLE_LOGGING: AtomicBool = AtomicBool::new(false);

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

pub fn init_alloc() {
    let mut log_messages = LOG_MESSAGES.lock();
    TEMPORARILY_DISABLE_LOGGING.store(true, Ordering::Release);
    *log_messages = log_messages.clone_in_global();
    TEMPORARILY_DISABLE_LOGGING.store(false, Ordering::Release);
    // LOG_MESSAGES_ALLOCATOR.pre_reserved_pages
}
