use common::syscall_start_recording_keyboard::{
    FullQueueBehavior, SyscallStartRecordingKeyboardInput,
};

use crate::syscall::{
    syscall_disable_and_defer_my_interrupts, syscall_done_with_interrupt_handler,
    syscall_enable_and_catch_up_on_my_interrupts,
    syscall_enable_my_interrupts_and_wait_until_one_happens, syscall_print,
    syscall_set_keyboard_interrupt_handler, syscall_start_recording_keyboard,
};

/// This is used to make sure that enabling and disabling interrupt works
pub fn test_disable_interrupts() -> ! {
    syscall_disable_and_defer_my_interrupts();
    syscall_start_recording_keyboard(SyscallStartRecordingKeyboardInput {
        queue_size: 256,
        behavior_on_full_queue: FullQueueBehavior::DropNewest,
    });
    syscall_set_keyboard_interrupt_handler(Some(keyboard_interrupt_handler));
    syscall_print("Interrupts disabled").unwrap();
    for _ in 0..50_000_000 {}
    syscall_enable_and_catch_up_on_my_interrupts();
    syscall_print("Interrupts enabled").unwrap();
    loop {
        syscall_enable_my_interrupts_and_wait_until_one_happens();
    }
}

unsafe extern "sysv64" fn keyboard_interrupt_handler() -> ! {
    // We cannot allocate during the interrupt handler because that would cause a lock forever.
    // Same reason why we can't allocate in kernel interrupt handlers
    syscall_print("Keyboard interrupt").unwrap();
    // for _ in 0..1_000_000 {}
    syscall_done_with_interrupt_handler();
}
