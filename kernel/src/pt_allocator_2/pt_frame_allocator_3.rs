use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

use super::PHYS_MEM_TRACKER;

pub struct PtFrameAllocator3<F: FnMut(PhysFrame<Size4KiB>)> {
    pub f: F,
}
unsafe impl<F: FnMut(PhysFrame<Size4KiB>)> FrameAllocator<Size4KiB> for PtFrameAllocator3<F> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let mut phys_mem = PHYS_MEM_TRACKER.try_get().unwrap().lock();
        let start = phys_mem.get_continuous_range_with_alignment(false, 0x1000, 0x1000)?;
        phys_mem.set(start..start + 0x1000, true);
        let phys_frame = PhysFrame::from_start_address(PhysAddr::new(start as u64)).unwrap();
        (self.f)(phys_frame);
        Some(phys_frame)
    }
}
