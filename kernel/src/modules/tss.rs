use x86_64::{structures::tss::TaskStateSegment, VirtAddr};

pub struct TssBuilder {
    used_interrupt_stack_table_entries: usize,
    used_privelage_stack_table_entries: usize,
    tss: TaskStateSegment,
}

impl Default for TssBuilder {
    fn default() -> Self {
        Self {
            used_interrupt_stack_table_entries: 0,
            used_privelage_stack_table_entries: 0,
            tss: TaskStateSegment::new(),
        }
    }
}

impl TssBuilder {
    /// Returns `None` if all entries are used
    pub fn add_interrupt_stack_table_entry(&mut self, address: VirtAddr) -> Option<usize> {
        // The interrupt stack table has 7 slots
        if self.used_interrupt_stack_table_entries < 7 {
            let index = self.used_interrupt_stack_table_entries;
            self.tss.interrupt_stack_table[index] = address;
            self.used_interrupt_stack_table_entries += 1;
            Some(index)
        } else {
            None
        }
    }

    pub fn add_privilege_stack_table_entry(&mut self, address: VirtAddr) -> Option<usize> {
        if self.used_privelage_stack_table_entries < 3 {
            self.tss.privilege_stack_table[self.used_privelage_stack_table_entries] = address;
            let r = Some(self.used_privelage_stack_table_entries);
            self.used_privelage_stack_table_entries += 1;
            r
        } else {
            None
        }
    }

    pub fn get_tss(self) -> TaskStateSegment {
        self.tss
    }
}
