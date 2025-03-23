use limine::response::MemoryMapResponse;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

pub struct TempFrameAllocator<'a> {
    pub used_phys_bytes: &'a mut usize,
    pub memory_map_response: &'a MemoryMapResponse,
    pub just_allocated_frames: &'a mut heapless::Vec<PhysFrame<Size4KiB>, 3>,
}
unsafe impl FrameAllocator<Size4KiB> for TempFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let start = {
            let mut bytes_to_subtract = *self.used_phys_bytes;
            self.memory_map_response.entries().iter().find_map(|entry| {
                let bytes_subtracted = bytes_to_subtract.min(entry.length as usize);
                let available_len = entry.length as usize - bytes_subtracted;
                bytes_to_subtract -= bytes_subtracted;
                if available_len >= 0x1000 {
                    Some(entry.base as usize + bytes_subtracted)
                } else {
                    None
                }
            })
        }?;
        *self.used_phys_bytes += 0x1000;
        let phys_frame = PhysFrame::from_start_address(PhysAddr::new(start as u64)).unwrap();
        self.just_allocated_frames.push(phys_frame).unwrap();
        Some(phys_frame)
    }
}
