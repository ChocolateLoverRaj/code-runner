use core::alloc::Layout;

use alloc::{
    alloc::{alloc, dealloc, realloc},
    boxed::Box,
    vec::Vec,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

pub fn test_allocator(max_size: usize) {
    {
        log::info!("Testing a simple allocation of 1 byte");
        let mut b = Box::new(3_u8);
        assert_eq!(*b, 3);
        *b += 5;
        assert_eq!(*b, 8);
        log::info!("Dropping (deallocating)");
        drop(b);
    }

    {
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
    }

    {
        log::info!("Allocating maximum amount of memory possible again");
        let b = Box::<[u8]>::new_uninit_slice(max_size);
        log::info!("Doing large allocation again");
        drop(b);
    }

    {
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
        for item in &v {
            assert_eq!(*item, 1);
        }
        assert_eq!(v[original_capacity], 2);
        drop(v);
    }

    {
        log::info!("Testing shrink");
        let layout = Layout::from_size_align(2, 1).unwrap();
        let ptr = unsafe { alloc(layout) };
        unsafe { core::slice::from_raw_parts_mut(ptr, 2) }.copy_from_slice(&[1, 2]);
        let ptr = unsafe { realloc(ptr, layout, 1) };
        assert_eq!(unsafe { ptr.read() }, 1);
        let new_layout = Layout::from_size_align(1, 1).unwrap();
        unsafe { dealloc(ptr, new_layout) };
    }

    {
        log::info!("Testing large shrink");
        let layout = Layout::from_size_align(0x3000, 1).unwrap();
        let ptr = unsafe { alloc(layout) };
        unsafe { core::slice::from_raw_parts_mut(ptr, layout.size()) }.fill(1);
        let new_layout = Layout::from_size_align(0x1800, layout.align()).unwrap();
        let ptr = unsafe { realloc(ptr, layout, new_layout.size()) };
        log::info!("Ptr: {:?}", ptr);
        log::info!("New ptr: {:?}", ptr);
        assert!(
            unsafe { core::slice::from_raw_parts_mut(ptr, new_layout.size()) }
                .iter()
                .copied()
                .eq(core::iter::repeat_n(1, new_layout.size()))
        );
        unsafe { dealloc(ptr, new_layout) };
    }

    {
        log::info!("Testing realloc with relocation");
        let layout = Layout::from_size_align(8, 1).unwrap();
        let ptr = unsafe { alloc(layout) };
        unsafe { core::slice::from_raw_parts_mut(ptr, 8) }.fill(1);
        let ptr2 = unsafe { alloc(layout) };
        let new_layout = Layout::from_size_align(0x2000, 1).unwrap();
        let ptr = unsafe { realloc(ptr, layout, new_layout.size()) };
        assert!(unsafe { core::slice::from_raw_parts_mut(ptr, 8) }
            .iter()
            .copied()
            .eq(core::iter::repeat_n(1, 8)));
        unsafe {
            dealloc(ptr2, layout);
            dealloc(ptr, new_layout);
        }
    }
}
