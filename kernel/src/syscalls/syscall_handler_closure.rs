use common::syscall_uuids::{
    ListenAction, Syscall, SyscallExists, SyscallExit, SyscallListenForKeyboardInterrupts,
    SyscallListenForKeyboardInterruptsOutputError, SyscallLog, SyscallTest, SyscallWaitUntilEvent,
};
use x2apic::ioapic::{IrqMode, RedirectionTableEntry};

use crate::{
    cpu_local_data::get_local,
    init_idt_and_gdt::MAPPED_APICS,
    pic8259_interrupts::Pic8259Interrupts,
    run_tasks::run_tasks,
    tasks::{KeyboardEventListener, TaskState, TaskType, TASKS},
    terminate_current_task::terminate_current_task,
};

use super::{
    raw_syscall_handler::SyscallHandlerClosure, syscall_handlers::SyscallHandlers,
    take_io_port::setup_take_io_port,
};

pub fn get_syscall_handlers() -> impl SyscallHandlerClosure {
    let mut syscall_handlers = SyscallHandlers::default();
    syscall_handlers.insert::<SyscallTest>(|input, _, _| {
        if input == SyscallTest::TEST_INPUT {
            log::info!("Test syscall with expected input: {:?}", input);
        } else {
            log::warn!("Test syscall had unexpected input: {:?}", input);
        }
        SyscallTest::TEST_OUTPUT
    });
    syscall_handlers.insert::<SyscallExit>(|_, _, _| {
        log::info!("Syscall exit called");
        terminate_current_task()
    });
    syscall_handlers
        .insert::<SyscallExists>(|uuid, _, syscall_handlers| syscall_handlers.contains_key(&uuid));
    syscall_handlers.insert::<SyscallLog>(|message, _, _| {
        // FIXME: Check pointer
        let message = core::str::from_utf8(unsafe { message.to_slice() }).unwrap();
        log::info!("User space says {:?}", message);
    });
    setup_take_io_port(&mut syscall_handlers);
    syscall_handlers.insert::<SyscallListenForKeyboardInterrupts>(|listen_action, _, _| {
        enum Action {
            Return(<SyscallListenForKeyboardInterrupts as Syscall>::Output),
            Terminate,
        }
        let action = {
            let mut tasks = TASKS.try_get().unwrap().lock();
            let keyboard_listener_set = tasks.keyboard_listener.is_some();
            let current_task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
            let current_task = tasks
                .tasks
                .iter_mut()
                .find(|task| task.id == current_task_id)
                .unwrap();
            match &current_task.task_type {
                TaskType::User(data) => {
                    if data.permissions.keyboard_interrupts {
                        match listen_action {
                            ListenAction::StartListening => {
                                if !keyboard_listener_set {
                                    tasks.keyboard_listener = Some(KeyboardEventListener {
                                        task_id: current_task_id,
                                        pending_interrupt_received: false
                                    });
                                    let cpu_local_data = get_local().unwrap();
                                    let mut entry = RedirectionTableEntry::default();
                                    entry.set_vector(cpu_local_data.static_stuff2.try_get().unwrap().keyboard_interrupt_index);
                                    entry.set_mode(IrqMode::Fixed);
                                    entry.set_dest(cpu_local_data.lapic_id.try_into().unwrap());
                                    let mut io_apic = MAPPED_APICS.try_get().unwrap().io_apic.lock();
                                    log::info!("Enabled interrupts and set IO APIC entry: {:?}", entry);
                                    unsafe {  io_apic.set_table_entry(Pic8259Interrupts::Keyboard.into(), entry) };
                                    unsafe { io_apic.enable_irq(Pic8259Interrupts::Keyboard.into()) };
                                    Action::Return(Ok(()))
                                } else {
                                    Action::Return(Err(SyscallListenForKeyboardInterruptsOutputError::InUse))
                                }
                            },
                            ListenAction::StopListening => {
                                if keyboard_listener_set {
                                    tasks.keyboard_listener = None;
                                    let mut io_apic = MAPPED_APICS.try_get().unwrap().io_apic.lock();
                                    unsafe { io_apic.disable_irq(Pic8259Interrupts::Keyboard.into()) };
                                    Action::Return(Ok(()))
                                } else {
                                    Action::Return(Err(SyscallListenForKeyboardInterruptsOutputError::NotListening))
                                }
                            }
                        }
                    } else {
                        log::warn!("Task {} tried to listen for keyboard interrupts when it is not allowed to. Terminating.", current_task_id);
                        Action::Terminate
                    }
                }
            }
        };
        match action {
            Action::Return(r) => r,
            Action::Terminate => terminate_current_task(),
        }
    });
    syscall_handlers.insert::<SyscallWaitUntilEvent>(|_, pushed_registers, _| {
        enum Action {
            Return(<SyscallWaitUntilEvent as Syscall>::Output),
            RunTasks,
        }
        let action = {
            let mut tasks = TASKS.try_get().unwrap().lock();
            let current_task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
            match &mut tasks.keyboard_listener {
                Some(keyboard_event_listener) => {
                    if keyboard_event_listener.task_id == current_task_id {
                        if keyboard_event_listener.pending_interrupt_received {
                            keyboard_event_listener.pending_interrupt_received = false;
                            Action::Return(())
                        } else {
                            let current_task = tasks
                                .tasks
                                .iter_mut()
                                .find(|task| task.id == current_task_id)
                                .unwrap();
                            current_task.state = TaskState::WaitingUntilEvent(*pushed_registers);
                            Action::RunTasks
                        }
                    } else {
                        Action::Return(())
                    }
                }
                None => Action::Return(()),
            }
        };
        match action {
            Action::Return(r) => r,
            Action::RunTasks => run_tasks(),
        }
    });
    syscall_handlers
}
