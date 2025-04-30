use common::syscall_uuids::{
    ListenAction, Syscall, SyscallListenForKeyboard, SyscallListenForKeyboardInterruptsOutputError,
};
use x2apic::ioapic::{IrqMode, RedirectionTableEntry};

use crate::{
    cpu_local_data::get_local,
    interrupt_numbers::InterruptNumbers,
    mapped_apics::MAPPED_APICS,
    pic8259_interrupts::Pic8259Interrupts,
    tasks::{KeyboardEventListener, KEYBOARD_LISTENER, TASKS},
    terminate_current_task::terminate_current_task,
};

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallListenForKeyboardHandler;
impl SyscallHandler2 for SyscallListenForKeyboardHandler {
    type Syscall = SyscallListenForKeyboard;

    fn handle_syscall(
        &self,
        listen_action: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        enum Action {
            Return(<SyscallListenForKeyboard as Syscall>::Output),
            Terminate,
        }
        let action = {
            let tasks = TASKS.lock();
            let mut keyboard_listener = KEYBOARD_LISTENER.lock();
            // let keyboard_listener_set = tasks.keyboard_listener.is_some();
            let task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
            let task = tasks.get(&task_id).unwrap();
            if task.permissions.keyboard_interrupts {
                match listen_action {
                    ListenAction::StartListening => {
                        if keyboard_listener.is_none() {
                            *keyboard_listener = Some(KeyboardEventListener {
                                task_id,
                                pending_interrupt_received: false,
                            });
                            let cpu_local_data = get_local().unwrap();
                            let mut entry = RedirectionTableEntry::default();
                            entry.set_vector(InterruptNumbers::Keyboard.into());
                            entry.set_mode(IrqMode::Fixed);
                            entry.set_dest(cpu_local_data.lapic_id.try_into().unwrap());
                            let mut io_apic = MAPPED_APICS.try_get().unwrap().io_apic.lock();
                            log::info!("Enabled interrupts and set IO APIC entry: {:?}", entry);
                            unsafe {
                                io_apic.set_table_entry(Pic8259Interrupts::Keyboard.into(), entry)
                            };
                            unsafe { io_apic.enable_irq(Pic8259Interrupts::Keyboard.into()) };
                            Action::Return(Ok(()))
                        } else {
                            Action::Return(Err(
                                SyscallListenForKeyboardInterruptsOutputError::InUse,
                            ))
                        }
                    }
                    ListenAction::StopListening => {
                        if keyboard_listener.is_some() {
                            *keyboard_listener = None;
                            let mut io_apic = MAPPED_APICS.try_get().unwrap().io_apic.lock();
                            unsafe { io_apic.disable_irq(Pic8259Interrupts::Keyboard.into()) };
                            Action::Return(Ok(()))
                        } else {
                            Action::Return(Err(
                                SyscallListenForKeyboardInterruptsOutputError::NotListening,
                            ))
                        }
                    }
                }
            } else {
                log::warn!("Task {:?} tried to listen for keyboard interrupts when it is not allowed to. Terminating.", task_id);
                Action::Terminate
            }
        };
        match action {
            Action::Return(r) => r,
            Action::Terminate => terminate_current_task(),
        }
    }
}
