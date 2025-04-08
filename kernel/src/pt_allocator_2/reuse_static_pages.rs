use core::alloc::Layout;

use alloc::alloc::dealloc;

use super::pre_reserved_pages::PreReservedPages;

/// Marks `static` pages are available to use, so that they are not wasted once the allocator is set up.
/// If you are transitioning from `static`ally reserved memory to dynamically allocated memory, you can use this function to clean up the `static` memory after switching to the global allocator.
pub fn reuse_static_pages<const N: usize>(pre_reserved_pages: &'static mut PreReservedPages<N>) {
    let ptr = pre_reserved_pages.bytes.as_ptr();
    assert!(ptr.is_aligned_to(0x1000));
    let ptr = ptr.cast_mut().cast();
    let layout = Layout::from_size_align(N, 0x1000).unwrap();
    unsafe { dealloc(ptr, layout) };
}
