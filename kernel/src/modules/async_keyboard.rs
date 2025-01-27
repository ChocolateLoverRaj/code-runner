use core::{
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::sync::Arc;
use conquer_once::noblock::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream};
use spin::Mutex;
use x2apic::{
    ioapic::{IoApic, RedirectionTableEntry},
    lapic::LocalApic,
};
use x86_64::{
    instructions::port::Port,
    structures::idt::{self, HandlerFunc, InterruptStackFrame},
};

use crate::pic8259_interrupts::Pic8259Interrupts;

use super::{idt::IdtBuilder, local_apic_getter::LocalApicGetter};

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();
static GETTER: OnceCell<&'static OnceCell<Mutex<LocalApic>>> = OnceCell::uninit();

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    match SCANCODE_QUEUE.try_get() {
        Ok(queue) => match queue.push(scancode) {
            Ok(()) => {
                WAKER.wake();
            }
            Err(e) => {
                log::warn!("WARNING: scancode queue full; dropping keyboard input: {e:?}");
            }
        },
        Err(e) => {
            log::warn!("WARNING: Could not add scancode: {e:?}");
        }
    }
    let mut local_apic = GETTER.try_get().unwrap().try_get().unwrap().lock();
    unsafe { local_apic.end_of_interrupt() };
}

unsafe fn enable_interrupts(io_apic: &mut IoApic) {
    log::debug!("Enabling keyboard interrupts");
    io_apic.enable_irq(Pic8259Interrupts::Keyboard.into())
}
unsafe fn disable_interrupts(io_apic: &mut IoApic) {
    log::debug!("Disabling keyboard interrupts");
    io_apic.disable_irq(Pic8259Interrupts::Keyboard.into())
}

pub struct AsyncKeyboardBuilder {
    interrupt_index: u8,
}

impl AsyncKeyboardBuilder {
    pub fn set_interrupt(idt_builder: &mut IdtBuilder) -> Option<Self> {
        let interrupt_index = idt_builder.set_flexible_entry({
            let mut entry = idt::Entry::<HandlerFunc>::missing();
            entry.set_handler_fn(keyboard_interrupt_handler);
            entry
        })?;
        Some(Self { interrupt_index })
    }

    pub fn configure_io_apic(
        &'static self,
        io_apic: Arc<Mutex<IoApic>>,
        local_apic_getter: &'static OnceCell<Mutex<LocalApic>>,
        queue_size: usize,
    ) -> AsyncKeyboard {
        GETTER.try_init_once(|| local_apic_getter).unwrap();
        {
            let mut io_apic = io_apic.lock();
            unsafe {
                io_apic.set_table_entry(Pic8259Interrupts::Keyboard.into(), {
                    let mut entry = RedirectionTableEntry::default();
                    entry.set_vector(self.interrupt_index);
                    entry
                })
            };
            unsafe {
                disable_interrupts(&mut io_apic);
            }
        }
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(queue_size))
            .unwrap();
        AsyncKeyboard { io_apic }
    }
}

pub struct AsyncKeyboard {
    io_apic: Arc<Mutex<IoApic>>,
}

impl AsyncKeyboard {
    pub fn stream(&mut self) -> ScancodeStream {
        let scancode_queue = SCANCODE_QUEUE.try_get().unwrap();
        loop {
            let popped_value = scancode_queue.pop();
            if popped_value.is_none() {
                break;
            }
        }
        unsafe { enable_interrupts(self.io_apic.lock().deref_mut()) };
        ScancodeStream {
            io_apic: self.io_apic.clone(),
        }
    }
}

pub struct ScancodeStream {
    io_apic: Arc<Mutex<IoApic>>,
}

impl Drop for ScancodeStream {
    fn drop(&mut self) {
        unsafe { disable_interrupts(self.io_apic.lock().deref_mut()) };
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}
