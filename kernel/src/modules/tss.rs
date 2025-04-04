use x86_64::{structures::tss::TaskStateSegment, VirtAddr};

#[derive(Debug)]
pub struct TssBuilder<const N: usize> {
    used_interrupt_stack_table_entries: u16,
    set_privilege_stack_table_entry_from_ring_3: bool,
    pub tss: TaskStateSegment<N>,
}

impl<const N: usize> Default for TssBuilder<N> {
    fn default() -> Self {
        Self {
            used_interrupt_stack_table_entries: 0,
            set_privilege_stack_table_entry_from_ring_3: false,
            tss: Default::default(),
        }
    }
}

impl<const N: usize> TssBuilder<N> {
    /// Returns `None` if all entries are used
    pub fn add_interrupt_stack_table_entry(&mut self, address: VirtAddr) -> Option<u16> {
        // The interrupt stack table has 7 slots
        if self.used_interrupt_stack_table_entries < 7 {
            let index = self.used_interrupt_stack_table_entries;
            self.tss.interrupt_stack_table[index as usize] = address;
            self.used_interrupt_stack_table_entries += 1;
            Some(index)
        } else {
            None
        }
    }

    /// Set the privilege stack table entry that tells the CPU which stack to switch to when an interrupt causes a switch from ring 3 to ring 0.
    /// The other privilege stack table entries are for transitioning from ring 1 and ring 2. Since in 2025 no one uses ring 1 or 2, those can be ignored.
    /// If this entry is already set, returns `Err` with the provided address.
    pub fn set_privilege_stack_table_entry_from_ring_3(
        &mut self,
        address: VirtAddr,
    ) -> Result<(), VirtAddr> {
        if !self.set_privilege_stack_table_entry_from_ring_3 {
            self.tss.privilege_stack_table[0] = address;
            self.set_privilege_stack_table_entry_from_ring_3 = true;
            Ok(())
        } else {
            Err(address)
        }
    }

    pub fn get_tss(self) -> TaskStateSegment<N> {
        self.tss
    }
}
