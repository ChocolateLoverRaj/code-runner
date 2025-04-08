use core::{
    cell::RefCell,
    ops::{Deref, DerefMut, Range},
};
use std::collections::HashMap;

use chashmap::{CHashMap, ReadGuard};
use mock_bytes::MockBytes;
use mock_bytes_mut::MockBytesMut;
use rangemap::{RangeMap, RangeSet};

use crate::{
    allocator_test::{Memory, PageTable, PageTableEntry},
    get_table_indexes::get_page_table_indices,
    usable_frames_iterator::UsableFramesIterator,
};

mod mock_bytes;
mod mock_bytes_mut;

#[derive(Debug, Default)]
struct MockPageTable {
    entries: HashMap<usize, PageTableEntry>,
}

type RawFrame = [u8; 0x1000];

#[derive(Debug)]
enum PhysicalFrame {
    Raw(RawFrame),
    PageTable(MockPageTable),
}

impl PhysicalFrame {
    pub fn as_raw(&self) -> Option<&RawFrame> {
        match self {
            PhysicalFrame::Raw(raw) => Some(raw),
            _ => None,
        }
    }

    pub fn as_raw_mut(&mut self) -> Option<&mut RawFrame> {
        match self {
            PhysicalFrame::Raw(raw) => Some(raw),
            _ => None,
        }
    }

    pub fn as_page_table(&self) -> Option<&MockPageTable> {
        match self {
            PhysicalFrame::Raw(_) => None,
            PhysicalFrame::PageTable(table) => Some(table),
        }
    }

    pub fn as_page_table_mut(&mut self) -> Option<&mut MockPageTable> {
        match self {
            PhysicalFrame::Raw(_) => None,
            PhysicalFrame::PageTable(table) => Some(table),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Accesses {
    Read(usize),
    Write,
}

impl Accesses {
    fn is_read(&self) -> bool {
        match self {
            Accesses::Read(_) => true,
            _ => false,
        }
    }

    fn is_write(&self) -> bool {
        match self {
            Accesses::Write => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct MockMemory {
    usable_memory_regions: RangeSet<usize>,
    top_level_page_table: usize,
    /// A hash map of physical frame by starting address to physical frame data
    physical_frames: CHashMap<usize, PhysicalFrame>,
    physical_memory_references: RefCell<RangeMap<usize, Accesses>>,
    virtual_memory_references: RefCell<RangeMap<usize, Accesses>>,
}

impl MockMemory {
    /// Returns `Self` with the offset map start address, and returns `None` if there is not enough usable physical memory.
    pub fn new_with_offset_map(
        mut usable_memory_regions: RangeSet<usize>,
    ) -> Option<(Self, usize)> {
        let original_usable_memory_regions = usable_memory_regions.clone();

        // Create the page tables
        let mut use_frame_region = || {
            let frame_start =
                UsableFramesIterator::new(usable_memory_regions.iter().cloned()).next()?;
            usable_memory_regions.remove(frame_start..frame_start + 0x1000);
            Some(frame_start)
        };

        let physical_frames = CHashMap::default();

        // Create the top level page table
        let top_level_page_table_address = use_frame_region()?;
        let mut top_level_page_table = MockPageTable {
            entries: Default::default(),
        };

        let offset_map_start = 0;

        // Offset map all memory
        let mut map_frame = |virtual_page_start: usize, physical_frame_start: usize| {
            let (l4_index, l3_index, l2_index, l1_index, offset) =
                get_page_table_indices(virtual_page_start);
            assert_eq!(offset, 0);
            let mut binding = match top_level_page_table.entries.get_mut(&l4_index) {
                Some(entry) => {
                    assert!(!entry.huge);
                    physical_frames
                        .get_mut(&entry.physical_frame_start)
                        .unwrap()
                }
                None => {
                    let l3_address = use_frame_region().unwrap();
                    let l3 = PhysicalFrame::PageTable(Default::default());
                    physical_frames.insert(l3_address, l3);
                    top_level_page_table.entries.insert(
                        l4_index,
                        PageTableEntry {
                            huge: false,
                            physical_frame_start: l3_address,
                        },
                    );
                    physical_frames.get_mut(&l3_address).unwrap()
                }
            };
            let l3_table = binding.as_page_table_mut().unwrap();
            let mut binding = match l3_table.entries.get(&l3_index) {
                Some(entry) => {
                    assert!(!entry.huge);
                    let physical_frame = entry.physical_frame_start;
                    physical_frames.get_mut(&physical_frame).unwrap()
                }
                None => {
                    let l2_address = use_frame_region().unwrap();
                    let l2 = PhysicalFrame::PageTable(Default::default());
                    l3_table.entries.insert(
                        l3_index,
                        PageTableEntry {
                            huge: false,
                            physical_frame_start: l2_address,
                        },
                    );
                    physical_frames.insert(l2_address, l2);
                    physical_frames.get_mut(&l2_address).unwrap()
                }
            };
            let l2_table = binding.as_page_table_mut().unwrap();
            let mut binding = match l2_table.entries.get(&l2_index) {
                Some(entry) => {
                    assert!(!entry.huge);
                    let physical_frame = entry.physical_frame_start;
                    physical_frames.get_mut(&physical_frame).unwrap()
                }
                None => {
                    let l1_address = use_frame_region().unwrap();
                    let l1 = PhysicalFrame::PageTable(Default::default());
                    l2_table.entries.insert(
                        l2_index,
                        PageTableEntry {
                            huge: false,
                            physical_frame_start: l1_address,
                        },
                    );
                    physical_frames.insert(l1_address, l1);
                    physical_frames.get_mut(&l1_address).unwrap()
                }
            };
            let l1_table = binding.as_page_table_mut().unwrap();
            l1_table.entries.insert(
                l1_index,
                PageTableEntry {
                    huge: false,
                    physical_frame_start,
                },
            )
        };

        original_usable_memory_regions
            .into_iter()
            .for_each(|region| {
                let mut physical_frame_start = region.start.div_floor(0x1000) * 0x1000;
                while physical_frame_start < region.end {
                    map_frame(
                        physical_frame_start + offset_map_start,
                        physical_frame_start,
                    );
                    physical_frame_start += 0x1000;
                }
            });

        physical_frames.insert(
            top_level_page_table_address,
            PhysicalFrame::PageTable(top_level_page_table),
        );

        Some((
            Self {
                usable_memory_regions,
                top_level_page_table: top_level_page_table_address,
                physical_frames,
                physical_memory_references: Default::default(),
                virtual_memory_references: Default::default(),
            },
            offset_map_start,
        ))
    }

    fn get_frame_physical_address(&self, virtual_address: usize) -> Option<(usize, usize)> {
        let (l4_index, l3_index, l2_index, l1_index, offset) =
            get_page_table_indices(virtual_address);
        let physical_frames = &self.physical_frames;
        let l4_table = physical_frames.get(&self.top_level_page_table)?;
        let l3_table = physical_frames.get(
            &l4_table
                .as_page_table()?
                .entries
                .get(&l4_index)?
                .physical_frame_start,
        )?;
        let l2_table = physical_frames.get(
            &l3_table
                .as_page_table()?
                .entries
                .get(&l3_index)?
                .physical_frame_start,
        )?;
        let l1_table = physical_frames.get(
            &l2_table
                .as_page_table()?
                .entries
                .get(&l2_index)?
                .physical_frame_start,
        )?;
        Some((
            l1_table
                .as_page_table()?
                .entries
                .get(&l1_index)?
                .physical_frame_start,
            offset,
        ))
    }

    fn get_physical_address(&self, virtual_address: usize) -> Option<usize> {
        let (frame_address, offset) = self.get_frame_physical_address(virtual_address)?;
        Some(frame_address + offset)
    }

    /// Creates a zeroed raw frame if the frame does not exist
    fn ensure_raw_frame(&self, frame_start: usize) {
        if !self.physical_frames.contains_key(&frame_start) {
            self.physical_frames.insert(
                frame_start,
                PhysicalFrame::Raw([Default::default(); 0x1000]),
            );
        }
    }
}

impl Memory for MockMemory {
    fn usable_physical_memory_regions(&self) -> impl Iterator<Item = Range<usize>> {
        self.usable_memory_regions.iter().cloned()
    }

    fn get_current_top_level_page_table(&self) -> usize {
        self.top_level_page_table
    }

    fn get_page_table(&self, page_table_virtual_address: usize) -> Option<impl PageTable> {
        let (frame_address, offset) =
            self.get_frame_physical_address(page_table_virtual_address)?;
        assert_eq!(offset, 0);
        let page_table = self.physical_frames.get(&frame_address)?;
        Some(TestPageTable { page_table })
    }

    fn get_bytes(&self, virtual_address_range: Range<usize>) -> impl Deref<Target = [u8]> {
        MockBytes::new(self, virtual_address_range)
    }

    fn get_bytes_mut(&self, virtual_address_range: Range<usize>) -> impl DerefMut<Target = [u8]> {
        MockBytesMut::new(self, virtual_address_range)
    }
}

pub struct TestPageTable<'a> {
    page_table: ReadGuard<'a, usize, PhysicalFrame>,
}

impl PageTable for TestPageTable<'_> {
    fn present_entries(&self) -> impl Iterator<Item = (usize, PageTableEntry)> {
        self.page_table
            .as_page_table()
            .unwrap()
            .entries
            .iter()
            .map(|(index, entry)| (*index, *entry))
    }

    fn get_entry(&self, index: usize) -> Option<PageTableEntry> {
        self.page_table
            .as_page_table()
            .unwrap()
            .entries
            .get(&index)
            .copied()
    }
}
