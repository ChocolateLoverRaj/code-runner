use common::syscall_start_recording_keyboard::{
    FullQueueBehavior, SyscallStartRecordingKeyboardInput,
};

use crate::syscall::{
    syscall_block_until_event, syscall_disable_my_interrupts, syscall_done_with_interrupt_handler,
    syscall_enable_my_interrupts, syscall_print, syscall_set_keyboard_interrupt_handler,
    syscall_start_recording_keyboard,
};

pub fn test_disable_interrupts() -> ! {
    syscall_disable_my_interrupts();
    syscall_start_recording_keyboard(SyscallStartRecordingKeyboardInput {
        queue_size: 256,
        behavior_on_full_queue: FullQueueBehavior::DropNewest,
    });
    syscall_set_keyboard_interrupt_handler(Some(keyboard_interrupt_handler));
    for _ in 0..10_000_000 {}
    syscall_enable_my_interrupts();
    loop {
        syscall_block_until_event();
    }
}

unsafe extern "sysv64" fn keyboard_interrupt_handler() -> ! {
    // We cannot allocate during the interrupt handler because that would cause a lock forever.
    // Same reason why we can't allocate in kernel interrupt handlers
    syscall_print("Keyboard interrupt").unwrap();
    // for _ in 0..1_000_000 {}
    syscall_done_with_interrupt_handler();
}
