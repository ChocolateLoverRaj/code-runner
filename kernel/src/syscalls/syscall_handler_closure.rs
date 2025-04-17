use common::syscall_uuids::{
    ListenAction, Syscall, SyscallExists, SyscallListenForKeyboardInterrupts,
    SyscallListenForKeyboardInterruptsOutputError, SyscallTest, SyscallWaitUntilEvent,
};
use limine::response::FramebufferResponse;
use x2apic::ioapic::{IrqMode, RedirectionTableEntry};

use crate::{
    cpu_local_data::get_local,
    hhdm_offset::HhdmOffset,
    init_idt_and_gdt::MAPPED_APICS,
    pic8259_interrupts::Pic8259Interrupts,
    run_tasks::run_tasks,
    tasks::{KeyboardEventListener, SavedSyscallState, TaskState, TaskType, TASKS},
    terminate_current_task::terminate_current_task,
};

use super::{
    exit::SyscallExitHandler,
    log::SyscallLogHandler,
    raw_syscall_handler::SyscallHandlerClosure,
    screen::{SyscallReleaseScreenHandler, SyscallTakeScreenHandler},
    syscall_handlers::SyscallHandlers,
    take_io_port::setup_take_io_port,
};

pub fn get_syscall_handlers(
    hhdm_offset: HhdmOffset,
    frame_buffer: Option<&'static FramebufferResponse>,
) -> impl SyscallHandlerClosure {
    let mut syscall_handlers = SyscallHandlers::default();
    syscall_handlers.insert::<SyscallTest>(|input, _, _| {
        if input == SyscallTest::TEST_INPUT {
            log::info!("Test syscall with expected input: {:?}", input);
        } else {
            log::warn!("Test syscall had unexpected input: {:?}", input);
        }
        SyscallTest::TEST_OUTPUT
    });
    syscall_handlers.insert_2(SyscallExitHandler);
    syscall_handlers
        .insert::<SyscallExists>(|uuid, _, syscall_handlers| syscall_handlers.contains_key(&uuid));
    syscall_handlers.insert_2(SyscallLogHandler { hhdm_offset });
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
            let cpu_local_data = get_local().unwrap();
            let current_task_id = cpu_local_data.task_data.lock().current_task.unwrap();
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
                            current_task.state = TaskState::WaitingUntilEvent(SavedSyscallState {
                                pushed_registers: *pushed_registers,
                                stack_pointer: unsafe {
                                    cpu_local_data.user_stack_pointer.get().read()
                                },
                            });
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
    syscall_handlers.insert_2(SyscallTakeScreenHandler::new(hhdm_offset, frame_buffer));
    syscall_handlers.insert_2(SyscallReleaseScreenHandler::new(hhdm_offset, frame_buffer));
    syscall_handlers
}
