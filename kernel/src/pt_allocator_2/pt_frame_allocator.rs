use alloc::vec::Vec;
use util::continuous_bool_vec::ContinuousBoolVec;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

pub struct PtFrameAllocator<'a> {
    pub phys: &'a mut ContinuousBoolVec<Vec<usize>>,
}
unsafe impl FrameAllocator<Size4KiB> for PtFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let start = self
            .phys
            .get_continuous_range_with_alignment(false, 0x1000, 0x1000)?;
        self.phys.set(start..start + 0x1000, true);
        let phys_frame = PhysFrame::from_start_address(PhysAddr::new(start as u64)).unwrap();
        log::debug!("Allocating phys frame: {:?}", phys_frame);
        Some(phys_frame)
    }
}
