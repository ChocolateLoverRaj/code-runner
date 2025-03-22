use alloc::boxed::Box;

pub fn test_allocator(max_size: usize) {
    log::info!("Testing a simple allocation of 1 byte");
    let b = Box::new(3_u8);
    log::info!("Dropping (deallocating)");
    drop(b);

    log::info!("Testing maximum amount of memory possible to allocate");
    let b = Box::<[u8]>::new_uninit_slice(max_size);
    log::info!("Dropping large allocation");
    drop(b);

    log::info!("Allocating maximum amount of memory possible again");
    let b = Box::<[u8]>::new_uninit_slice(max_size);
    log::info!("Dropping large allocation again");
    drop(b);
}
