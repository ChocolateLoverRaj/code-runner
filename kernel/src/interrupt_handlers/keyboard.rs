use core::{
    arch::naked_asm,
    cmp::Ordering,
    mem::{self, MaybeUninit},
};

use alloc::collections::btree_map::BTreeMap;
use common::syscall_uuids::{serialize_output, EventId, SyscallWaitUntilEvent};
use x86_64::{
    registers::{control::Cr3, model_specific::GsBase, segmentation::GS},
    structures::{gdt::SegmentSelector, idt::InterruptStackFrame},
    PrivilegeLevel,
};

use crate::{
    context::{Context, FullContext, SyscallContext},
    cpu_local_data::{get_local, CpuLocalData},
    run_tasks::update_iobp,
    tasks::{CpuTaskData, Task, TaskId, TaskState, KEYBOARD_LISTENER, TASKS, TASK_QUEUE},
};

/// # Safety
/// This function should only be called as a keyboard interrupt handler, and not manually.
#[naked]
pub unsafe extern "sysv64" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        naked_asm!("\
            push r15
            push r14
            push r13
            push r12
            push r11
            push r10
            push r9
            push r8
            push rdi
            push rsi
            push rdx
            push rcx
            push rbx
            push rax
            push rbp

            mov rdi, rsp   // first arg of context switch is the context which is all the registers saved above

            // The function should never return
            call {context_switch}
            // asm! version of unreachable!()
            ud2
            ",
            context_switch = sym keyboard_interrupt_handler_rust
        );
    };
}

unsafe extern "sysv64" fn keyboard_interrupt_handler_rust(context: &FullContext) -> ! {
    // This is to make sure that GS.Base is set to the kernel's gs base
    let swapped_gs =
        if SegmentSelector(context.cs.try_into().unwrap()).rpl() == PrivilegeLevel::Ring3 {
            unsafe { GS::swap() };
            log::trace!("Swapped GS");
            true
        } else {
            log::trace!("Didn't swap GS");
            false
        };

    enum Action {
        RestoreSaved(SyscallContext),
        RestoreFull,
    }
    let action = {
        let cpu_local_data = get_local().unwrap();
        unsafe {
            cpu_local_data
                .local_apic
                .try_get()
                .unwrap()
                .lock()
                .end_of_interrupt()
        };
        let mut tasks = TASKS.lock();
        let mut cpu_task_data = cpu_local_data.task_data.lock();
        let task_queue = TASK_QUEUE.lock();
        let current_task_index = cpu_task_data.current_task.map(|current_task_id| {
            (
                current_task_id,
                task_queue
                    .iter()
                    .position(|id| id == &current_task_id)
                    .unwrap(),
            )
        });
        let mut keyboard_listener = KEYBOARD_LISTENER.lock();
        let keyboard_listener_task_id = keyboard_listener.as_ref().unwrap().task_id;
        let keyboard_listener_task_index = task_queue
            .iter()
            .position(|id| id == &keyboard_listener_task_id)
            .unwrap();
        log::debug!(
            "Current task id: {:?}. Keyboard listener task id: {:?}",
            cpu_task_data.current_task,
            keyboard_listener_task_id
        );
        match current_task_index {
            Some((current_task_id, current_task_index)) => {
                match keyboard_listener_task_index.cmp(&current_task_index) {
                    Ordering::Less => {
                        // Keyboard listener task is higher priority than the current task
                        // Interrupt the current task and switch to the keyboard listener task
                        let current_task = tasks.get_mut(&current_task_id).unwrap();
                        current_task.state = TaskState::Interrupted(*context);
                        Action::RestoreSaved(switch_to_task_that_received_event(
                            &mut tasks,
                            keyboard_listener_task_id,
                            &mut cpu_task_data,
                            &cpu_local_data,
                        ))
                    }
                    Ordering::Equal | Ordering::Greater => {
                        // The task is already running, or it's less priority
                        // Mark the keyboard event as happened
                        // Continue executing the current task
                        keyboard_listener
                            .as_mut()
                            .unwrap()
                            .pending_interrupt_received = true;
                        Action::RestoreFull
                    }
                }
            }
            None => Action::RestoreSaved(switch_to_task_that_received_event(
                &mut tasks,
                keyboard_listener_task_id,
                &mut cpu_task_data,
                &cpu_local_data,
            )),
        }
    };

    match action {
        Action::RestoreSaved(context) => {
            log::debug!("Restoring saved. GsBase now: {:?}", GsBase::read());
            // GS must be set to user mode's GS
            unsafe { GS::swap() };
            unsafe { context.restore() }
        }
        Action::RestoreFull => {
            log::debug!("Restoring full: {:#?}", context);
            if swapped_gs {
                unsafe { GS::swap() };
            }
            unsafe { context.restore() }
        }
    }
}

fn switch_to_task_that_received_event(
    tasks: &mut BTreeMap<TaskId, Task>,
    keyboard_listener_task_id: TaskId,
    cpu_task_data: &mut CpuTaskData,
    cpu_local_data: &CpuLocalData,
) -> SyscallContext {
    let keyboard_listener_task = tasks.get_mut(&keyboard_listener_task_id).unwrap();
    let waiting_until_event_data =
        match mem::replace(&mut keyboard_listener_task.state, TaskState::Running) {
            TaskState::WaitingUntilEvent(waiting_until_event_data) => waiting_until_event_data,
            state => unreachable!("Unexpected state: {:#?}", state),
        };
    let cr3_flags = Cr3::read().1;
    unsafe { Cr3::write(keyboard_listener_task.cr3, cr3_flags) };
    cpu_task_data.current_task = Some(keyboard_listener_task_id);
    let kernel_stack_pointer = keyboard_listener_task.kernel_stack.top().as_u64();
    unsafe {
        cpu_local_data
            .kernel_stack_pointer
            .get()
            .write(kernel_stack_pointer)
    };
    keyboard_listener_task.state = TaskState::Running;
    let events_that_happened = unsafe {
        waiting_until_event_data
            .input
            .to_slice_mut::<MaybeUninit<EventId>>()
    };
    if let Some(first_slot) = events_that_happened.first_mut() {
        first_slot.write(EventId::Keyboard);
    }
    update_iobp(keyboard_listener_task_id);
    SyscallContext::from_syscall_output(
        &waiting_until_event_data.state,
        // If there were other pending events, they would've already been handled.
        serialize_output::<SyscallWaitUntilEvent>(&1).unwrap(),
    )
}
