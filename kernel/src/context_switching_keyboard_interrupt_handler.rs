use core::arch::naked_asm;

use alloc::sync::Arc;
use conquer_once::noblock::OnceCell;
use spin::Mutex;
use x2apic::{
    ioapic::{IoApic, RedirectionTableEntry},
    lapic::LocalApic,
};
use x86_64::{
    instructions::port::Port,
    structures::idt::{self, HandlerFunc, InterruptStackFrame},
};

use crate::{
    context::Context, modules::idt::IdtBuilder, pic8259_interrupts::Pic8259Interrupts,
    restore_context::restore_context,
};

static LOCAL_APIC: OnceCell<&'static OnceCell<Mutex<LocalApic>>> = OnceCell::uninit();

#[naked]
unsafe extern "sysv64" fn context_switching_keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
    unsafe {
        naked_asm!("\
            push r15 
            push r14
            push r13
            push r12
            push r11
            push r10
            push r9
            push r8
            push rdi
            push rsi
            push rdx
            push rcx
            push rbx
            push rax
            push rbp
            mov rdi, rsp   // first arg of context switch is the context which is all the registers saved above
            sub rsp, 0x400
            jmp {context_switch}
            ", 
            context_switch = sym context_switching_keyboard_interrupt_handler_rust
        );
    };
}

unsafe extern "sysv64" fn context_switching_keyboard_interrupt_handler_rust(
    context: *const Context,
) {
    let context = unsafe { (*context).clone() };
    {
        let mut port = Port::new(0x60);
        let scancode: u8 = unsafe { port.read() };
        log::info!("Got scancode: {}", scancode);
        let mut local_apic = LOCAL_APIC.try_get().unwrap().try_get().unwrap().lock();
        unsafe { local_apic.end_of_interrupt() };
    }
    unsafe { restore_context(&context) };
}

unsafe fn enable_interrupts(io_apic: &mut IoApic) {
    log::debug!("Enabling keyboard interrupts");
    unsafe { io_apic.enable_irq(Pic8259Interrupts::Keyboard.into()) }
}

pub struct TestKeyboardBuilder {
    interrupt_index: u8,
}

impl TestKeyboardBuilder {
    pub fn set_interrupt(
        idt_builder: &mut IdtBuilder,
        local_apic: &'static OnceCell<Mutex<LocalApic>>,
    ) -> Option<Self> {
        LOCAL_APIC.try_init_once(|| local_apic).unwrap();
        let handler = unsafe {
            core::mem::transmute::<*const (), HandlerFunc>(
                context_switching_keyboard_interrupt_handler as *const _,
            )
        };
        let interrupt_index = idt_builder.set_flexible_entry({
            let mut entry = idt::Entry::<HandlerFunc>::missing();
            entry.set_handler_fn(handler);
            entry
        })?;
        Some(Self { interrupt_index })
    }

    pub fn configure_io_apic(&'static self, io_apic: Arc<Mutex<IoApic>>) {
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
                enable_interrupts(&mut io_apic);
            }
        }
    }
}
