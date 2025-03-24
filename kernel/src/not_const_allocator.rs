use core::alloc::GlobalAlloc;

use util::init_later::InitLater;

#[derive(Debug)]
pub struct NotConstAllocator<A> {
    actual_allocator: InitLater<A>,
}

impl<A> NotConstAllocator<A> {
    pub const fn uninit() -> Self {
        Self {
            actual_allocator: InitLater::uninit(),
        }
    }
    pub fn try_init(&self, val: A) -> Result<(), ()> {
        self.actual_allocator.try_init(val)?;
        Ok(())
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
