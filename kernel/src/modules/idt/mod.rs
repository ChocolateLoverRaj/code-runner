mod disable_pic8259;

use disable_pic8259::disable_pic8259;
use x86_64::structures::idt::{
    self, DivergingHandlerFuncWithErrCode, HandlerFunc, HandlerFuncWithErrCode,
    InterruptDescriptorTable, PageFaultHandlerFunc,
};

use crate::interrupts::{
    keyboard::keyboard_interrupt_handler, rtc::rtc_interrupt_handler, InterruptIndex,
};

pub struct IdtBuilder {
    idt: InterruptDescriptorTable,
    set_double_fault_entry: bool,
    set_breakpoint_entry: bool,
    set_general_protection_fault: bool,
    set_page_fault_entry: bool,
}

impl IdtBuilder {
    pub fn new() -> Self {
        Self {
            idt: {
                let mut idt = InterruptDescriptorTable::new();

                idt[u8::from(InterruptIndex::Timer)].set_handler_fn(rtc_interrupt_handler);
                idt[u8::from(InterruptIndex::Keyboard)].set_handler_fn(keyboard_interrupt_handler);
                idt[u8::from(InterruptIndex::Rtc)].set_handler_fn(rtc_interrupt_handler);

                idt
            },
            set_double_fault_entry: false,
            set_breakpoint_entry: false,
            set_general_protection_fault: false,
            set_page_fault_entry: false,
        }
    }

    pub fn set_double_fault_entry(
        &mut self,
        entry: idt::Entry<DivergingHandlerFuncWithErrCode>,
    ) -> Result<(), ()> {
        if !self.set_double_fault_entry {
            self.idt.double_fault = entry;
            self.set_double_fault_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn set_breakpoint_entry(&mut self, entry: idt::Entry<HandlerFunc>) -> Result<(), ()> {
        if !self.set_breakpoint_entry {
            self.idt.breakpoint = entry;
            self.set_breakpoint_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn set_general_protection_fault_entry(
        &mut self,
        entry: idt::Entry<HandlerFuncWithErrCode>,
    ) -> Result<(), ()> {
        if !self.set_general_protection_fault {
            self.idt.general_protection_fault = entry;
            self.set_general_protection_fault = true;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn set_page_fault_entry(
        &mut self,
        entry: idt::Entry<PageFaultHandlerFunc>,
    ) -> Result<(), ()> {
        if !self.set_page_fault_entry {
            self.idt.page_fault = entry;
            self.set_page_fault_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn init(&'static self) {
        self.idt.load();
        disable_pic8259();
        x86_64::instructions::interrupts::enable();
    }
}
