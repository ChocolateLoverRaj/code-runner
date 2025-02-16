use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::{PageSize, Size4KiB};

use crate::syscall::syscall_allocate_pages;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// This function should only be called once
pub fn init() {
    // TODO: Allocate more pages if no pages
    let total_pages = 100;
    let start = syscall_allocate_pages(total_pages);
    let heap_size = Size4KiB::SIZE * total_pages;
    unsafe {
        ALLOCATOR
            .lock()
            .init(start.as_mut_ptr(), heap_size as usize)
    };
}
