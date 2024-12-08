mod disable_pic8259;

use core::u8;

use disable_pic8259::disable_pic8259;
use x86_64::structures::idt::{
    self, DivergingHandlerFuncWithErrCode, HandlerFunc, HandlerFuncWithErrCode,
    InterruptDescriptorTable, PageFaultHandlerFunc,
};

use crate::interrupts::{
    keyboard::keyboard_interrupt_handler, rtc::rtc_interrupt_handler, InterruptIndex,
};

const FLEXIBLE_ENTRIES_START: u8 = 32;
const MAX_FLEXIBLE_ENTRIES: u8 = FLEXIBLE_ENTRIES_START.wrapping_neg();

pub struct IdtBuilder {
    idt: InterruptDescriptorTable,
    set_double_fault_entry: bool,
    set_breakpoint_entry: bool,
    set_general_protection_fault: bool,
    set_page_fault_entry: bool,
    used_flexible_entries: [bool; MAX_FLEXIBLE_ENTRIES as usize],
}

impl IdtBuilder {
    pub fn new() -> Self {
        Self {
            idt: {
                let mut idt = InterruptDescriptorTable::new();

                // idt[u8::from(InterruptIndex::Timer)].set_handler_fn(rtc_interrupt_handler);
                // idt[u8::from(InterruptIndex::Keyboard)].set_handler_fn(keyboard_interrupt_handler);
                // idt[u8::from(InterruptIndex::Rtc)].set_handler_fn(rtc_interrupt_handler);

                idt
            },
            set_double_fault_entry: false,
            set_breakpoint_entry: false,
            set_general_protection_fault: false,
            set_page_fault_entry: false,
            used_flexible_entries: [false; MAX_FLEXIBLE_ENTRIES as usize],
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

    /// Finds an unused entry and assigns the interrupt handler to it.
    /// Returns the index if it found an unused entry.
    /// Returns `None` if all entries are used.
    pub fn set_flexible_entry(&mut self, entry: idt::Entry<HandlerFunc>) -> Option<u8> {
        let flexible_entry_index = self.used_flexible_entries.iter().position(|used| !used)?;
        let entry_index = flexible_entry_index as u8 + FLEXIBLE_ENTRIES_START;
        self.idt[entry_index] = entry;
        self.used_flexible_entries[flexible_entry_index] = true;
        Some(entry_index)
    }

    /// Set an entry for a specific index, returning `Err` if the index already has an entry set.
    pub fn set_fixed_entry(
        &mut self,
        entry_index: u8,
        entry: idt::Entry<HandlerFunc>,
    ) -> Result<(), ()> {
        let flexible_entry_index = entry_index - FLEXIBLE_ENTRIES_START;
        if !self.used_flexible_entries[flexible_entry_index as usize] {
            self.idt[entry_index] = entry;
            self.used_flexible_entries[flexible_entry_index as usize] = true;
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
