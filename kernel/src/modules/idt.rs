use core::cell::UnsafeCell;

use x86_64::{
    structures::idt::{self, DivergingHandlerFuncWithErrCode, InterruptDescriptorTable},
    VirtAddr,
};

use crate::interrupts::{
    breakpoint::breakpoint_handler, disable_pic8259::disable_pic8259,
    double_fault::double_fault_handler, general_protection_fault::general_protection_fault_handler,
    keyboard::keyboard_interrupt_handler, page_fault::page_fault_handler,
    rtc::rtc_interrupt_handler, InterruptIndex,
};

use super::{gtd::DOUBLE_FAULT_IST_INDEX, tss::TssBuilder};

pub struct IdtBuilder {
    idt: InterruptDescriptorTable,
}

impl IdtBuilder {
    pub fn new() -> Self {
        Self {
            idt: {
                let mut idt = InterruptDescriptorTable::new();
                // idt.breakpoint.set_handler_fn(breakpoint_handler);
                // {
                //     let entry_options = idt.double_fault.set_handler_fn(double_fault_handler);
                //     let stack_index = tss
                //         .add_interrupt_stack_table_entry({
                //             const STACK_SIZE: usize = 4096 * 5;
                //             const STACK: UnsafeCell<[u8; STACK_SIZE]> =
                //                 UnsafeCell::new([0; STACK_SIZE]);

                //             let stack_start = VirtAddr::from_ptr(STACK.get());
                //             let stack_end = stack_start + STACK_SIZE as u64;
                //             stack_end
                //         })
                //         .unwrap();
                //     unsafe {
                //         entry_options.set_stack_index(stack_index as u16);
                //     }
                // }
                idt.general_protection_fault
                    .set_handler_fn(general_protection_fault_handler);
                idt.page_fault.set_handler_fn(page_fault_handler);

                idt[u8::from(InterruptIndex::Timer)].set_handler_fn(rtc_interrupt_handler);
                idt[u8::from(InterruptIndex::Keyboard)].set_handler_fn(keyboard_interrupt_handler);
                idt[u8::from(InterruptIndex::Rtc)].set_handler_fn(rtc_interrupt_handler);

                idt
            },
        }
    }

    pub fn add_double_fault_handler(&mut self, entry: idt::Entry<DivergingHandlerFuncWithErrCode>) {
        self.idt.double_fault = entry;
    }

    pub fn init(&'static self) {
        self.idt.load();
        disable_pic8259();
        x86_64::instructions::interrupts::enable();
    }
}
