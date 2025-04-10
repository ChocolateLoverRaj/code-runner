use util::x86_64_allocator::GetTestableAllocator;

use super::LOCKED_ALLOCATOR;

pub struct LockedAllocator;

impl GetTestableAllocator for LockedAllocator {
    fn get_testable_allocator(
        &self,
    ) -> impl core::ops::DerefMut<Target = impl util::testable_allocator::TestableAllocator> {
        LOCKED_ALLOCATOR.try_get().unwrap().lock()
    }
}
