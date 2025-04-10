use core::{alloc::GlobalAlloc, ops::DerefMut};

use crate::testable_allocator::TestableAllocator;

pub trait GetTestableAllocator {
    fn get_testable_allocator(&self) -> impl DerefMut<Target = impl TestableAllocator>;
}

pub struct X86_64Allocator<T> {
    get_testable_allocator: T,
}

impl<T> X86_64Allocator<T> {
    /// # Safety
    /// The testable allocator must represent real memory
    pub const unsafe fn new(testable_allocator: T) -> Self {
        Self {
            get_testable_allocator: testable_allocator,
        }
    }
}

unsafe impl<T: GetTestableAllocator> GlobalAlloc for X86_64Allocator<T> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        match self
            .get_testable_allocator
            .get_testable_allocator()
            .allocate(layout)
        {
            Some(ptr) => usize::from(ptr) as *mut u8,
            None => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.get_testable_allocator
            .get_testable_allocator()
            .deallocate((ptr as usize).try_into().unwrap(), layout);
    }
}
