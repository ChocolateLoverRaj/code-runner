use x86_64::{structures::tss::TaskStateSegment, VirtAddr};

/// `N` is the size of the IOBP in **bytes**
/// Do not change `iomap_base`. It is set to the correct value by the `default` function.
/// By default, the IOPB is set to all 1s, so that no port access is allowed
// #[derive(Debug, Clone, Copy)]
// #[repr(C, packed(4))]
// pub struct TssWithIoBitMap<const N: usize> {
//     pub above_iobp: TaskStateSegment,
//     pub actual_io_bitmap: [u8; N],
//     /// According to Section 20.5.2 in *Intel® 64 and IA-32 Architectures Software Developer’s Manual*
//     /// > Last byte of bitmap must be followed by a byte with all bits set.
//     io_bit_map_last_byte: u8,
// }

// impl<const N: usize> Default for TssWithIoBitMap<N> {
//     fn default() -> Self {
//         Self {
//             above_iobp: {
//                 let mut tss = TaskStateSegment::default();
//                 // tss.iomap_base = (offset_of!(Self, actual_io_bitmap)).try_into().unwrap();
//                 tss
//             },
//             // If a bit is 0 it means the port is allowed, 1 is not allowed
//             // actual_io_bitmap: [u8::MAX; N],
//             actual_io_bitmap: [u8::MAX; N],
//             io_bit_map_last_byte: u8::MAX,
//         }
//     }
// }

#[derive(Debug)]
pub struct TssBuilder<const N: usize> {
    used_interrupt_stack_table_entries: u16,
    used_privilege_stack_table_entries: usize,
    pub tss: TaskStateSegment<N>,
}

impl<const N: usize> Default for TssBuilder<N> {
    fn default() -> Self {
        Self {
            used_interrupt_stack_table_entries: 0,
            used_privilege_stack_table_entries: 0,
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

    pub fn add_privilege_stack_table_entry(&mut self, address: VirtAddr) -> Option<usize> {
        if self.used_privilege_stack_table_entries < 3 {
            self.tss.privilege_stack_table[self.used_privilege_stack_table_entries] = address;
            let r = Some(self.used_privilege_stack_table_entries);
            self.used_privilege_stack_table_entries += 1;
            r
        } else {
            None
        }
    }

    pub fn get_tss(self) -> TaskStateSegment<N> {
        self.tss
    }
}
