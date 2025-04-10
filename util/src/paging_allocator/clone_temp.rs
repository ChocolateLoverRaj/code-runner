use crate::{continuous_bool_vec::ContinuousBoolVec, static_allocator::StaticAllocator};
use alloc::vec::Vec;

pub fn clone_temp(
    physical_memory_tracker: ContinuousBoolVec<Vec<usize, StaticAllocator>>,
    virtual_memory_tracker: ContinuousBoolVec<Vec<usize, StaticAllocator>>,
) -> (ContinuousBoolVec<Vec<usize>>, ContinuousBoolVec<Vec<usize>>) {
    // Because the way our temp allocator is compatible with the current allocator, we can simply switch allocators
    let phys_mem_tracker = physical_memory_tracker.replace_len_vec(|len_vec| {
        let (a, b, c) = Vec::into_parts(len_vec);
        unsafe { Vec::from_parts(a, b, c) }
    });
    let kernel_address_space_tracker = virtual_memory_tracker.replace_len_vec(|len_vec| {
        let (a, b, c) = Vec::into_parts(len_vec);
        unsafe { Vec::from_parts(a, b, c) }
    });
    (phys_mem_tracker, kernel_address_space_tracker)
}
