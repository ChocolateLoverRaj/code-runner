use core::{alloc::Layout, num::NonZeroUsize};

pub trait TestableAllocator {
    fn allocate(&mut self, layout: Layout) -> Option<NonZeroUsize>;
    fn deallocate(&mut self, ptr: NonZeroUsize, layout: Layout);
}
