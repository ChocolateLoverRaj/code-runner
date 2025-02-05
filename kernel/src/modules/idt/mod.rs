mod disable_pic8259;

use disable_pic8259::disable_pic8259;
use x86_64::structures::idt::{
    self, DivergingHandlerFuncWithErrCode, HandlerFunc, HandlerFuncWithErrCode,
    InterruptDescriptorTable, PageFaultHandlerFunc,
};

const FLEXIBLE_ENTRIES_START: u8 = 32;
const MAX_FLEXIBLE_ENTRIES: u8 = FLEXIBLE_ENTRIES_START.wrapping_neg();

pub struct IdtBuilder {
    idt: InterruptDescriptorTable,
    set_double_fault_entry: bool,
    set_breakpoint_entry: bool,
    set_general_protection_fault: bool,
    set_page_fault_entry: bool,
    set_invalid_tss_fault_entry: bool,
    set_security_exception_fault_entry: bool,
    set_segment_not_present_entry: bool,
    set_invalid_opcode_entry: bool,
    set_stack_segment_fault_entry: bool,
    used_flexible_entries: [bool; MAX_FLEXIBLE_ENTRIES as usize],
}

impl Default for IdtBuilder {
    fn default() -> Self {
        Self {
            idt: InterruptDescriptorTable::new(),
            set_double_fault_entry: false,
            set_breakpoint_entry: false,
            set_general_protection_fault: false,
            set_page_fault_entry: false,
            set_invalid_tss_fault_entry: false,
            set_security_exception_fault_entry: false,
            set_segment_not_present_entry: false,
            set_invalid_opcode_entry: false,
            set_stack_segment_fault_entry: false,
            used_flexible_entries: [false; MAX_FLEXIBLE_ENTRIES as usize],
        }
    }
}

impl IdtBuilder {
    #[allow(clippy::result_unit_err)]
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

    #[allow(clippy::result_unit_err)]
    pub fn set_breakpoint_entry(&mut self, entry: idt::Entry<HandlerFunc>) -> Result<(), ()> {
        if !self.set_breakpoint_entry {
            self.idt.breakpoint = entry;
            self.set_breakpoint_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    #[allow(clippy::result_unit_err)]
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

    #[allow(clippy::result_unit_err)]
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

    #[allow(clippy::result_unit_err)]
    pub fn set_invalid_tss_fault_entry(
        &mut self,
        entry: idt::Entry<HandlerFuncWithErrCode>,
    ) -> Result<(), ()> {
        if !self.set_invalid_tss_fault_entry {
            self.idt.invalid_tss = entry;
            self.set_invalid_tss_fault_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn set_security_exception_fault_entry(
        &mut self,
        entry: idt::Entry<HandlerFuncWithErrCode>,
    ) -> Result<(), ()> {
        if !self.set_security_exception_fault_entry {
            self.idt.security_exception = entry;
            self.set_security_exception_fault_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn set_segment_not_present_entry(
        &mut self,
        entry: idt::Entry<HandlerFuncWithErrCode>,
    ) -> Result<(), ()> {
        if !self.set_segment_not_present_entry {
            self.idt.segment_not_present = entry;
            self.set_segment_not_present_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn set_invalid_opcode_entry(&mut self, entry: idt::Entry<HandlerFunc>) -> Result<(), ()> {
        if !self.set_invalid_opcode_entry {
            self.idt.invalid_opcode = entry;
            self.set_invalid_opcode_entry = true;
            Ok(())
        } else {
            Err(())
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn set_stack_segment_fault_entry(
        &mut self,
        entry: idt::Entry<HandlerFuncWithErrCode>,
    ) -> Result<(), ()> {
        if !self.set_stack_segment_fault_entry {
            self.idt.stack_segment_fault = entry;
            self.set_stack_segment_fault_entry = true;
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

    #[allow(clippy::result_unit_err)]
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
    }
}
