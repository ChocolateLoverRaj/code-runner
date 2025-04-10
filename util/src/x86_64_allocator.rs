use core::{alloc::GlobalAlloc, ops::DerefMut};

use crate::testable_allocator::TestableAllocator;

pub trait GetTestableAllocator<T: DerefMut<Target = dyn TestableAllocator>> {
    fn get_testable_allocator(&self) -> T;
}

pub struct X86_64Allocator<T: DerefMut<Target = dyn TestableAllocator>, F: Fn() -> T> {
    f: F,
    // allocator: Option<T>,
}

impl<T: DerefMut<Target = dyn TestableAllocator>, F: Fn() -> T> X86_64Allocator<T, F> {
    /// # Safety
    /// The testable allocator must represent real memory
    pub const unsafe fn new(f: F) -> Self {
        Self { f }
    }
}

unsafe impl<T: DerefMut<Target = dyn TestableAllocator>, F: Fn() -> T> GlobalAlloc
    for X86_64Allocator<T, F>
{
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
        // match self
        //     .get_testable_allocator
        //     .get_testable_allocator()
        //     .allocate(layout)
        // {
        //     Some(ptr) => usize::from(ptr) as *mut u8,
        //     None => core::ptr::null_mut(),
        // }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
        // self.get_testable_allocator
        //     .get_testable_allocator()
        //     .deallocate((ptr as usize).try_into().unwrap(), layout);
    }
}
