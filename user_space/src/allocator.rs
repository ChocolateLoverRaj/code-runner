use core::mem::MaybeUninit;

use linked_list_allocator::LockedHeap;

const GLOBAL_ALLOCATOR_SIZE: usize = 10 * 0x400;

static mut GLOBAL_ALLOCATOR_BYTES: [MaybeUninit<u8>; GLOBAL_ALLOCATOR_SIZE] =
    [MaybeUninit::uninit(); GLOBAL_ALLOCATOR_SIZE];

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// # Safety
/// This function must be called only once
pub unsafe fn init() {
    GLOBAL_ALLOCATOR
        .lock()
        .init_from_slice(unsafe { (&raw mut GLOBAL_ALLOCATOR_BYTES).as_mut() }.unwrap());
}
