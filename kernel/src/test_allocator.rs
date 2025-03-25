use core::{alloc::Layout, ptr::NonNull};

use alloc::{
    alloc::{alloc, realloc},
    boxed::Box,
    vec::Vec,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

pub fn test_allocator(max_size: usize) {
    log::info!("Testing a simple allocation of 1 byte");
    let mut b = Box::new(3_u8);
    assert_eq!(*b, 3);
    *b += 5;
    assert_eq!(*b, 8);
    log::info!("Dropping (deallocating)");
    drop(b);

    log::info!("Testing maximum amount of memory possible to allocate");
    log::info!("Doing large allocation");
    let mut v = Vec::<u8>::with_capacity(max_size);
    log::info!("Making sure same values are read back after writing");
    let rng = SmallRng::seed_from_u64(3);
    rng.clone()
        .random_iter::<u8>()
        .take(max_size)
        .collect_into(&mut v);
    assert!(v
        .iter()
        .copied()
        .eq(rng.clone().random_iter::<u8>().take(max_size)));
    drop(v);

    log::info!("Allocating maximum amount of memory possible again");
    let b = Box::<[u8]>::new_uninit_slice(max_size);
    log::info!("Doing large allocation again");
    drop(b);

    log::info!("Testing growing");
    let mut v = Vec::<u8>::with_capacity(1);
    let original_capacity = v.capacity();
    for _ in 0..original_capacity {
        v.push_within_capacity(1).unwrap();
    }
    log::info!(
        "Vec capacity: {}. Will increase by at least 1 to force realloc",
        v.capacity()
    );
    v.push(2);
    for i in 0..original_capacity {
        assert_eq!(v[i], 1);
    }
    assert_eq!(v[original_capacity], 2);
    log::info!("Vec: {:?}", v);
    drop(v);

    {
        log::info!("Testing shrink");
        let layout = Layout::from_size_align(2, 1).unwrap();
        let ptr = unsafe { alloc(layout) };
        NonNull::new(ptr).unwrap();
        unsafe { (ptr as *mut [u8; 2]).write([1, 2]) };
        unsafe { realloc(ptr, layout, 1) };
        assert_eq!(unsafe { ptr.read() }, 1);
    }
}
