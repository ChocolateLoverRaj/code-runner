use core::mem::MaybeUninit;

use linked_list_allocator::LockedHeap;

use crate::config::GLOBAL_ALLOCATOR_SIZE;

static mut GLOBAL_ALLOCATOR_BYTES: [MaybeUninit<u8>; GLOBAL_ALLOCATOR_SIZE] =
    [MaybeUninit::uninit(); GLOBAL_ALLOCATOR_SIZE];

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// # Safety
/// This function must be called only once
pub unsafe fn init() {
    GLOBAL_ALLOCATOR
        .lock()
        .init_from_slice(unsafe { &mut GLOBAL_ALLOCATOR_BYTES });
}
