use x86_64::{structures::tss::TaskStateSegment, VirtAddr};

pub struct TssBuilder {
    used_stack_table_entries: usize,
    tss: TaskStateSegment,
}

impl TssBuilder {
    pub fn new() -> Self {
        Self {
            used_stack_table_entries: 0,
            tss: TaskStateSegment::new(),
        }
    }

    /// Returns `None` if all entries are used
    pub fn add_interrupt_stack_table_entry(&mut self, address: VirtAddr) -> Option<usize> {
        // The interrupt stack table has 7 slots
        if self.used_stack_table_entries < 7 {
            self.tss.interrupt_stack_table[self.used_stack_table_entries] = address;
            Some(self.used_stack_table_entries)
        } else {
            None
        }
    }

    pub fn get_tss(self) -> TaskStateSegment {
        self.tss
    }
}
