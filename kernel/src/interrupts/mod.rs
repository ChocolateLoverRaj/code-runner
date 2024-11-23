mod breakpoint;
mod disable_pic8259;
mod double_fault;
mod general_protection_fault;
mod keyboard;
mod page_fault;
mod timer;

use breakpoint::breakpoint_handler;
use disable_pic8259::disable_pic8259;
use double_fault::double_fault_handler;
use general_protection_fault::general_protection_fault_handler;
use keyboard::keyboard_interrupt_handler;
use lazy_static::lazy_static;
use num_enum::IntoPrimitive;
use page_fault::page_fault_handler;
use timer::timer_interrupt_handler;
use x86_64::structures::idt::InterruptDescriptorTable;

use crate::gtd::DOUBLE_FAULT_IST_INDEX;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        {
            let entry_options = idt.double_fault.set_handler_fn(double_fault_handler);
            unsafe {
                entry_options.set_stack_index(DOUBLE_FAULT_IST_INDEX);
            }
        }
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt[u8::from(InterruptIndex::Timer)].set_handler_fn(timer_interrupt_handler);
        idt[u8::from(InterruptIndex::Keyboard)].set_handler_fn(keyboard_interrupt_handler);
        // idt[u8::from(InterruptIndex::Mouse)].set_handler_fn(mouse_interrupt_handler);

        idt
    };
}

pub fn init_interrupts() {
    IDT.load();
    disable_pic8259();
    x86_64::instructions::interrupts::enable();
}

#[derive(Debug, Clone, Copy, IntoPrimitive)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,
    Keyboard,
    LocalApicError,
    Suprious,
}
