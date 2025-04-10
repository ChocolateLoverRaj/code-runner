use util::paging_allocator::GetTrackers;

use super::{KERNEL_ADDRESS_SPACE_TRACKER, PHYS_MEM_TRACKER};

pub struct LockedMemTrackers;
impl GetTrackers for LockedMemTrackers {
    fn get_phys_tracker(
        &mut self,
    ) -> impl core::ops::DerefMut<
        Target = util::continuous_bool_vec::ContinuousBoolVec<alloc::vec::Vec<usize>>,
    > {
        PHYS_MEM_TRACKER.try_get().unwrap().lock()
    }

    fn get_virt_tracker(
        &mut self,
    ) -> impl core::ops::DerefMut<
        Target = util::continuous_bool_vec::ContinuousBoolVec<alloc::vec::Vec<usize>>,
    > {
        KERNEL_ADDRESS_SPACE_TRACKER.try_get().unwrap().lock()
    }
}
