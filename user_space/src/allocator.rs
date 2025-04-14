use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// /// This function should only be called once
// pub fn init() {
//     // TODO: Allocate more pages if no pages
//     let total_pages = 100;
//     let start = syscall_allocate_pages(total_pages);
//     let heap_size = Size4KiB::SIZE * total_pages;
//     unsafe {
//         ALLOCATOR
//             .lock()
//             .init(start.as_mut_ptr(), heap_size as usize)
//     };
// }
