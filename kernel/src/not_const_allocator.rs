use core::alloc::GlobalAlloc;

use conquer_once::noblock::OnceCell;

#[derive(Debug)]
pub struct NotConstAllocator<A> {
    actual_allocator: OnceCell<A>,
}

impl<A> NotConstAllocator<A> {
    pub const fn uninit() -> Self {
        Self {
            actual_allocator: OnceCell::uninit(),
        }
    }
    pub fn try_init_once(
        &self,
        func: impl FnOnce() -> A,
    ) -> Result<(), conquer_once::TryInitError> {
        self.actual_allocator.try_init_once(func)
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for NotConstAllocator<A> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe { self.actual_allocator.try_get().unwrap().alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        unsafe {
            self.actual_allocator
                .try_get()
                .unwrap()
                .dealloc(ptr, layout);
        }
    }

    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe { self.actual_allocator.try_get().unwrap().alloc(layout) }
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        unsafe {
            self.actual_allocator
                .try_get()
                .unwrap()
                .realloc(ptr, layout, new_size)
        }
    }
}
