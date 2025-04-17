use alloc::collections::btree_map;
use common::syscall_uuids::{
    IoPortAction, Syscall, SyscallTakeIoPort, SyscallTakeIoPortInput, SyscallTakeIoPortOutputError,
};

use crate::{
    cpu_local_data::get_local,
    init_idt_and_gdt::get_iopb,
    io_ports_lock::IO_PORT_USAGE,
    tasks::{TaskType, TASKS},
    terminate_current_task::terminate_current_task,
};

use super::syscall_handlers::SyscallHandlers;

pub fn setup_take_io_port(syscall_handlers: &mut SyscallHandlers) {
    // TODO: We are going to run into lock problems later
    syscall_handlers.insert::<SyscallTakeIoPort>(
        |SyscallTakeIoPortInput { action, port }, _, _| {
            enum Action {
                Return(<SyscallTakeIoPort as Syscall>::Output),
                Terminate,
            }
            let action = {
                let tasks = TASKS.try_get().unwrap().lock();
                let current_task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
                let current_task = tasks
                    .tasks
                    .iter()
                    .find(|task| task.id == current_task_id)
                    .unwrap();
                match &current_task.task_type {
                    TaskType::User(task_data) => {
                        let mut iopb = get_iopb().lock();
                        if iopb.contains_port_permission(port) {
                            let mut m = IO_PORT_USAGE.lock();
                            match action {
                                IoPortAction::Take => {
                                    if task_data.permissions.ports.contains(&port) {
                                        if let btree_map::Entry::Vacant(e) = m.entry(port) {
                                            e.insert(current_task_id);
                                            iopb.set_port_allowed(port, true);
                                            Action::Return(Ok(()))
                                        } else {
                                            Action::Return(Err(SyscallTakeIoPortOutputError::InUse))
                                        }
                                    } else {
                                        log::warn!("Process {} tried to access a io port it doesn't have permission for. Terminating.", current_task_id);
                                        Action::Terminate
                                    }
                                },
                                IoPortAction::Release => {
                                    match m.get(&port) {
                                        Some(id) => {
                                            if id == &current_task_id {
                                                m.remove(&port);
                                                Action::Return(Ok(()))
                                            } else {
                                                Action::Return(Err(SyscallTakeIoPortOutputError::NotOwned))
                                            }
                                        },
                                        None => {
                                            Action::Return(Err(SyscallTakeIoPortOutputError::NotOwned))
                                        }
                                    }
                                }
                            }

                        } else {
                            Action::Return(Err(SyscallTakeIoPortOutputError::OutOfIopb))
                        }
                    }
                }
            };
            match action {
                Action::Return(r) => r,
                Action::Terminate => terminate_current_task(),
            }
        },
    );
}
