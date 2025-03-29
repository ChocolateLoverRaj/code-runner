use alloc::vec::Vec;
use util::continuous_bool_vec::ContinuousBoolVec;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

pub struct PtFrameAllocator2<'a> {
    pub phys_mem: &'a mut ContinuousBoolVec<Vec<usize>>,
    pub used_bytes: &'a mut u64,
}
unsafe impl FrameAllocator<Size4KiB> for PtFrameAllocator2<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let start = self
            .phys_mem
            .get_continuous_range_with_alignment(false, 0x1000, 0x1000)?;
        self.phys_mem.set(start..start + 0x1000, true);
        *self.used_bytes += 0x1000;
        let phys_frame = PhysFrame::from_start_address(PhysAddr::new(start as u64)).unwrap();
        log::debug!("Allocating phys frame: {:?}", phys_frame);
        Some(phys_frame)
    }
}
