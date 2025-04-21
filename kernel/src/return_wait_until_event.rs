use core::mem::MaybeUninit;

use common::syscall_uuids::{EventId, Syscall, SyscallWaitUntilEvent};

use crate::tasks::{TaskId, KEYBOARD_LISTENER};

/// # Safety
/// Make sure that the input has been checked and the correct Cr3 is loaded.
pub unsafe fn handle_pending_events(
    task_id: TaskId,
    input: <SyscallWaitUntilEvent as Syscall>::Input,
) -> Option<<SyscallWaitUntilEvent as Syscall>::Output> {
    match &mut *KEYBOARD_LISTENER.lock() {
        Some(keyboard_event_listener) => {
            if keyboard_event_listener.task_id == task_id
                && keyboard_event_listener.pending_interrupt_received
            {
                keyboard_event_listener.pending_interrupt_received = false;
                let events_that_happened = unsafe { input.to_slice_mut::<MaybeUninit<EventId>>() };
                if let Some(first_slot) = events_that_happened.first_mut() {
                    first_slot.write(EventId::Keyboard);
                }
                Some(1)
            } else {
                None
            }
        }
        None => unreachable!("Shouldn't have waited for events"),
    }
}
