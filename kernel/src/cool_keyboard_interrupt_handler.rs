use core::ops::{Deref, DerefMut};

use alloc::sync::Arc;
use common::syscall_start_recording_keyboard::{
    FullQueueBehavior, SyscallStartRecordingKeyboardInput,
};
use conquer_once::noblock::OnceCell;
use crossbeam_queue::ArrayQueue;
use spin::{Mutex, RwLock, RwLockReadGuard};
use x2apic::{
    ioapic::{IoApic, RedirectionTableEntry},
    lapic::LocalApic,
};
use x86_64::{
    instructions::port::Port,
    structures::idt::{self, HandlerFunc, InterruptStackFrame},
};

use crate::{modules::idt::IdtBuilder, pic8259_interrupts::Pic8259Interrupts};

static LOCAL_APIC: OnceCell<&'static OnceCell<Mutex<LocalApic>>> = OnceCell::uninit();

struct RecordingKeyboard {
    pub full_queue_behavior: FullQueueBehavior,
    pub queue: ArrayQueue<u8>,
}

static SCANCODE_QUEUE: RwLock<Option<RecordingKeyboard>> = RwLock::new(None);

extern "x86-interrupt" fn cool_keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scan_code: u8 = unsafe { port.read() };
    if let Some(RecordingKeyboard {
        full_queue_behavior,
        queue,
    }) = SCANCODE_QUEUE.read().deref()
    {
        match full_queue_behavior {
            FullQueueBehavior::DropNewest => {
                let _ = queue.push(scan_code);
            }
            FullQueueBehavior::DropOldest => {
                queue.force_push(scan_code);
            }
        }
    };
    let mut local_apic = LOCAL_APIC.try_get().unwrap().try_get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}

unsafe fn enable_interrupts(io_apic: &mut IoApic) {
    unsafe { io_apic.enable_irq(Pic8259Interrupts::Keyboard.into()) }
}

unsafe fn disable_interrupts(io_apic: &mut IoApic) {
    unsafe { io_apic.disable_irq(Pic8259Interrupts::Keyboard.into()) }
}

pub struct CoolKeyboardBuilder {
    interrupt_index: u8,
}

impl CoolKeyboardBuilder {
    pub fn set_interrupt(
        idt_builder: &mut IdtBuilder,
        local_apic: &'static OnceCell<Mutex<LocalApic>>,
    ) -> Option<Self> {
        LOCAL_APIC.try_init_once(|| local_apic).unwrap();
        let interrupt_index = idt_builder.set_flexible_entry({
            let mut entry = idt::Entry::<HandlerFunc>::missing();
            entry.set_handler_fn(cool_keyboard_interrupt_handler);
            entry
        })?;
        Some(Self { interrupt_index })
    }

    pub fn configure_io_apic(&'static self, io_apic: Arc<Mutex<IoApic>>) -> CoolKeyboard {
        {
            let mut io_apic = io_apic.lock();
            unsafe {
                io_apic.set_table_entry(Pic8259Interrupts::Keyboard.into(), {
                    let mut entry = RedirectionTableEntry::default();
                    entry.set_vector(self.interrupt_index);
                    entry
                })
            };
        }
        CoolKeyboard { io_apic }
    }
}

#[derive(Debug, Clone)]
pub struct CoolKeyboard {
    io_apic: Arc<Mutex<IoApic>>,
}

impl CoolKeyboard {
    pub fn enable(&self, settings: SyscallStartRecordingKeyboardInput) {
        *SCANCODE_QUEUE.write() = Some(RecordingKeyboard {
            full_queue_behavior: settings.behavior_on_full_queue,
            queue: ArrayQueue::new(settings.queue_size as usize),
        });
        unsafe { enable_interrupts(self.io_apic.lock().deref_mut()) };
    }

    pub fn disable(&self) {
        unsafe { disable_interrupts(self.io_apic.lock().deref_mut()) };
    }

    pub fn queue(&self) -> QueueGuard {
        QueueGuard {
            guard: SCANCODE_QUEUE.read(),
        }
    }
}

pub struct QueueGuard<'a> {
    guard: RwLockReadGuard<'a, Option<RecordingKeyboard>>,
}

impl QueueGuard<'_> {
    pub fn queue(&self) -> Option<&ArrayQueue<u8>> {
        self.guard
            .as_ref()
            .map(|recording_keyboard| &recording_keyboard.queue)
    }
}
