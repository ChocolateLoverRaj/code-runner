use core::{cell::SyncUnsafeCell, mem::MaybeUninit};

use alloc::format;
use linked_list_allocator::LockedHeap;

use crate::syscall::syscall_print;

const GLOBAL_ALLOCATOR_SIZE: usize = 10 * 0x400;

static GLOBAL_ALLOCATOR_BYTES: SyncUnsafeCell<[MaybeUninit<u8>; GLOBAL_ALLOCATOR_SIZE]> =
    SyncUnsafeCell::new([MaybeUninit::uninit(); GLOBAL_ALLOCATOR_SIZE]);

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// # Safety
/// This function must be called only once
pub unsafe fn init() {
    GLOBAL_ALLOCATOR
        .lock()
        .init_from_slice(unsafe { GLOBAL_ALLOCATOR_BYTES.get().as_mut().unwrap() });
    syscall_print(&format!(
        "Initialized global allocator using memory at {:p}",
        GLOBAL_ALLOCATOR_BYTES.get()
    ));
}
