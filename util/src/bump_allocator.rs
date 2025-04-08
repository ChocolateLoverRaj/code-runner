use core::{
    alloc::{AllocError, Allocator},
    cell::RefCell,
    ptr::NonNull,
};

struct BumpAllocatorData<'a> {
    memory: &'a mut [u8],
    position: usize,
}

pub struct BumpAllocator<'a> {
    data: RefCell<BumpAllocatorData<'a>>,
}

impl<'a> From<&'a mut [u8]> for BumpAllocator<'a> {
    fn from(value: &'a mut [u8]) -> Self {
        BumpAllocator {
            data: RefCell::new(BumpAllocatorData {
                memory: value,
                position: 0,
            }),
        }
    }
}

unsafe impl Allocator for &BumpAllocator<'_> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let mut data = self.data.try_borrow_mut().map_err(|_| AllocError)?;
        let start = data.position.next_multiple_of(layout.align());
        let end = start + layout.size();
        if end <= data.memory.len() {
            data.position = end;
            Ok(NonNull::from_mut(&mut data.memory[start..end]))
        } else {
            Err(AllocError)
        }
    }

    unsafe fn deallocate(&self, _ptr: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        unimplemented!("This allocator is not meant to deallocate ever")
    }
}
