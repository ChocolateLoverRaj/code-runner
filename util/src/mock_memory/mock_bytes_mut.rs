use core::ops::{Deref, DerefMut, Range};

use crate::aligned_chunks::AlignedChunks;

use super::{Accesses, MockMemory};

pub struct MockBytesMut<'a> {
    bytes: Vec<u8>,
    virtual_address_range: Range<usize>,
    mock_memory: &'a MockMemory,
}

impl<'a> MockBytesMut<'a> {
    pub(crate) fn new(mock_memory: &'a MockMemory, virtual_address_range: Range<usize>) -> Self {
        let mut virtual_memory_references = mock_memory.virtual_memory_references.borrow_mut();
        if virtual_memory_references.overlaps(&virtual_address_range) {
            panic!("Cannot mutably reference virtual memory when the virtual memory is already referenced");
        };
        virtual_memory_references.insert(virtual_address_range.clone(), Accesses::Write);
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
                if physical_memory_references.overlaps(&physical_memory_range) {
                    panic!("Cannot mutably borrow physical memory that's already being borrowed");
                }
                physical_memory_references.insert(physical_memory_range, Accesses::Write);
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

impl Drop for MockBytesMut<'_> {
    fn drop(&mut self) {
        // Actually write the memory to the physical frames
        let mut bytes_updated = 0;
        let mut physical_memory_references =
            self.mock_memory.physical_memory_references.borrow_mut();
        self.virtual_address_range
            .clone()
            .aligned_chunks(0x1000)
            .for_each(|chunk| {
                let (frame_start, offset) = self
                    .mock_memory
                    .get_frame_physical_address(chunk.start)
                    .unwrap();
                let physical_memory_range = {
                    let physical_memory_start = frame_start + offset;
                    physical_memory_start..physical_memory_start + chunk.len()
                };
                self.mock_memory
                    .physical_frames
                    .get_mut(&frame_start)
                    .unwrap()
                    .as_raw_mut()
                    .unwrap()[offset..offset + chunk.len()]
                    .copy_from_slice(&self.bytes[bytes_updated..bytes_updated + chunk.len()]);
                physical_memory_references.remove(physical_memory_range);
                bytes_updated += chunk.len();
            });
        self.mock_memory
            .virtual_memory_references
            .borrow_mut()
            .remove(self.virtual_address_range.clone());
    }
}

impl Deref for MockBytesMut<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for MockBytesMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}
