use alloc::alloc::Allocator;
use core::alloc::GlobalAlloc;

pub struct StaticAllocator;

unsafe impl GlobalAlloc for StaticAllocator {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        unimplemented!()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        unimplemented!()
    }
}

unsafe impl Allocator for StaticAllocator {
    fn allocate(
        &self,
        _layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        unimplemented!()
    }

    unsafe fn deallocate(&self, _ptr: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        unimplemented!()
    }
}
