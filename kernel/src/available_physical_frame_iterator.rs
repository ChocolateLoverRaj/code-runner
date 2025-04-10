use core::{fmt::Debug, ops::Range};

use limine::{memory_map::EntryType, response::MemoryMapResponse};
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

pub struct AvailablePhysicalRegionsIterator {
    memory_map_response: &'static MemoryMapResponse,
    index: usize,
}

impl Debug for AvailablePhysicalRegionsIterator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AvailablePhysicalRegionsIterator")
            .field("memory_map_response", &"<!Debug>")
            .field("index", &self.index)
            .finish()
    }
}

impl From<&'static MemoryMapResponse> for AvailablePhysicalRegionsIterator {
    fn from(memory_map_response: &'static MemoryMapResponse) -> Self {
        Self {
            memory_map_response,
            index: Default::default(),
        }
    }
}

impl Iterator for AvailablePhysicalRegionsIterator {
    type Item = Range<PhysAddr>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.memory_map_response.entries().get(self.index)?;
            self.index += 1;
            if entry.entry_type == EntryType::USABLE {
                let start = PhysAddr::new(entry.base);
                break Some(start..start + entry.length);
            }
        }
    }
}

#[derive(Debug)]
pub struct AvailablePhysicalFrameIterator {
    available_physical_regions_iterator: AvailablePhysicalRegionsIterator,
    current_region: Option<(Range<PhysAddr>, PhysFrame<Size4KiB>)>,
}

impl From<AvailablePhysicalRegionsIterator> for AvailablePhysicalFrameIterator {
    fn from(available_physical_regions_iterator: AvailablePhysicalRegionsIterator) -> Self {
        Self {
            available_physical_regions_iterator,
            current_region: Default::default(),
        }
    }
}

impl Iterator for AvailablePhysicalFrameIterator {
    type Item = PhysFrame<Size4KiB>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (current_region, index) = match &mut self.current_region {
                Some(current_region) => current_region,
                None => self.current_region.insert({
                    let range = self.available_physical_regions_iterator.next()?;
                    let index =
                        PhysFrame::from_start_address(range.start.align_up(0x1000_u64)).unwrap();
                    (range, index)
                }),
            };
            if (*index + 1).start_address() <= current_region.end {
                *index += 1;
                break Some(*index);
            } else {
                self.current_region = None;
            }
        }
    }
}

#[derive(Debug)]
pub struct AvailablePhysicalFrameIteratorFrameAllocator {
    iterator: AvailablePhysicalFrameIterator,
}

impl AvailablePhysicalFrameIteratorFrameAllocator {
    /// # Safety
    /// You must not have already used the physical memory that the iterator says is available.
    pub const unsafe fn new(iterator: AvailablePhysicalFrameIterator) -> Self {
        Self { iterator }
    }
}

unsafe impl FrameAllocator<Size4KiB> for AvailablePhysicalFrameIteratorFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.iterator.next()
    }
}
