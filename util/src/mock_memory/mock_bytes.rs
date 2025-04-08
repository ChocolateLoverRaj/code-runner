use core::ops::{Deref, DerefMut, Range};

use crate::aligned_chunks::AlignedChunks;

use super::{Accesses, MockMemory};

pub struct MockBytes<'a> {
    bytes: Vec<u8>,
    virtual_address_range: Range<usize>,
    mock_memory: &'a MockMemory,
}

impl<'a> MockBytes<'a> {
    pub(crate) fn new(mock_memory: &'a MockMemory, virtual_address_range: Range<usize>) -> Self {
        let mut virtual_memory_references = mock_memory.virtual_memory_references.borrow_mut();
        if virtual_memory_references
            .overlapping(virtual_address_range.clone())
            .any(|(_range, accesses)| accesses.is_write())
        {
            panic!("Cannot immutably reference virtual memory when the virtual memory is already being referenced mutably");
        };
        virtual_memory_references
            .clone()
            .overlapping(virtual_address_range.clone())
            .for_each(|(range, accesses)| {
                let range = range.start.max(virtual_address_range.start)
                    ..range.end.min(virtual_address_range.end);
                virtual_memory_references.insert(
                    range,
                    match accesses {
                        Accesses::Read(count) => Accesses::Read(count + 1),
                        Accesses::Write => unreachable!(),
                    },
                );
            });
        let mut bytes = Vec::with_capacity(virtual_address_range.len());
        let mut physical_memory_references = mock_memory.physical_memory_references.borrow_mut();
        virtual_address_range
            .clone()
            .aligned_chunks(0x1000)
            .for_each(|chunk| {
                let (frame_start, offset) = mock_memory
                    .get_frame_physical_address(chunk.start)
                    .expect(&format!("Page not mapped: {:?}", chunk));
                let physical_memory_range = {
                    let physical_memory_start = frame_start + offset;
                    physical_memory_start..physical_memory_start + chunk.len()
                };
                if physical_memory_references.overlapping(physical_memory_range.clone()).any(|(_range, accesses)| accesses.is_write()) {
                   panic!("Cannot get an immutable reference to physical memory that is already being referenced mutably");
                }
                physical_memory_references.clone().overlapping(physical_memory_range.clone()).for_each(|(range, accesses)| {
                    let range = range.start.max(physical_memory_range.start)..range.end.min(physical_memory_range.end);
                    physical_memory_references.insert(range, match accesses {
                        Accesses::Read(count) => Accesses::Read(count + 1),
                        Accesses::Write => unreachable!()
                    });
                });
                mock_memory.ensure_raw_frame(frame_start);
                bytes.extend_from_slice(
                    &mock_memory
                        .physical_frames
                        .get(&frame_start)
                        .unwrap()
                        .as_raw()
                        .expect("Attempting to read a page table as raw bytes")
                        [offset..offset + chunk.len()],
                );
            });
        Self {
            bytes,
            mock_memory,
            virtual_address_range,
        }
    }
}

impl Drop for MockBytes<'_> {
    fn drop(&mut self) {
        let mut physical_memory_references =
            self.mock_memory.physical_memory_references.borrow_mut();
        self.virtual_address_range
            .clone()
            .aligned_chunks(0x1000)
            .for_each(|chunk| {
                let physical_memory_start =
                    self.mock_memory.get_physical_address(chunk.start).unwrap();
                let physical_memory_range =
                    physical_memory_start..physical_memory_start + chunk.len();
                physical_memory_references
                    .clone()
                    .overlapping(physical_memory_range.clone())
                    .for_each(|(range, accesses)| {
                        let range = range.start.max(physical_memory_range.start)
                            ..range.end.min(physical_memory_range.end);
                        let new_count = match accesses {
                            Accesses::Read(count) => count - 1,
                            Accesses::Write => unreachable!(),
                        };
                        if new_count == 0 {
                            physical_memory_references.remove(range.clone());
                        } else {
                            physical_memory_references
                                .insert(range.clone(), Accesses::Read(new_count));
                        }
                    });
            });
        let mut virtual_memory_references = self.mock_memory.virtual_memory_references.borrow_mut();
        virtual_memory_references
            .clone()
            .overlapping(self.virtual_address_range.clone())
            .for_each(|(range, accesses)| {
                let range = range.start.max(self.virtual_address_range.start)
                    ..range.end.min(self.virtual_address_range.end);
                let new_count = match accesses {
                    Accesses::Read(count) => count - 1,
                    Accesses::Write => unreachable!(),
                };
                if new_count == 0 {
                    virtual_memory_references.remove(range.clone());
                } else {
                    virtual_memory_references.insert(range.clone(), Accesses::Read(new_count));
                }
            });
    }
}

impl Deref for MockBytes<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for MockBytes<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}
