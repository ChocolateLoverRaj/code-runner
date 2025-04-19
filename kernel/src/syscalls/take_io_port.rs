use alloc::collections::btree_map;
use common::syscall_uuids::{
    IoPortAction, Syscall, SyscallTakeIoPort, SyscallTakeIoPortOutputError,
};

use crate::{
    cpu_local_data::get_local,
    init_idt_and_gdt::get_iopb,
    io_ports_lock::{IoPortUsedBy, IO_PORT_USAGE},
    tasks::TASKS,
    terminate_current_task::terminate_current_task,
};

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallTakeIoPortHandler;
impl SyscallHandler2 for SyscallTakeIoPortHandler {
    type Syscall = SyscallTakeIoPort;

    fn handle_syscall(
        &self,
        input: <Self::Syscall as Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as Syscall>::Output {
        enum Action {
            Return(<SyscallTakeIoPort as Syscall>::Output),
            Terminate,
        }
        let action = {
            let tasks = TASKS.lock();
            let task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
            let task = tasks.get(&task_id).unwrap();
            let mut iopb = get_iopb().lock();
            if iopb.contains_port_permission(input.port) {
                let mut io_port_usage = IO_PORT_USAGE.lock();
                match input.action {
                    IoPortAction::Take => {
                        if task.permissions.ports.contains(&input.port) {
                            if let btree_map::Entry::Vacant(e) = io_port_usage.entry(input.port) {
                                e.insert(IoPortUsedBy::User(task_id));
                                iopb.set_port_allowed(input.port, true);
                                Action::Return(Ok(()))
                            } else {
                                Action::Return(Err(SyscallTakeIoPortOutputError::InUse))
                            }
                        } else {
                            log::warn!("Process {:?} tried to access a io port it doesn't have permission for. Terminating.", task_id);
                            Action::Terminate
                        }
                    }
                    IoPortAction::Release => match io_port_usage.get(&input.port) {
                        Some(IoPortUsedBy::User(used_by_id)) => {
                            if used_by_id == &task_id {
                                io_port_usage.remove(&input.port);
                                Action::Return(Ok(()))
                            } else {
                                Action::Return(Err(SyscallTakeIoPortOutputError::NotOwned))
                            }
                        }
                        _ => Action::Return(Err(SyscallTakeIoPortOutputError::NotOwned)),
                    },
                }
            } else {
                Action::Return(Err(SyscallTakeIoPortOutputError::OutOfIopb))
            }
        };
        match action {
            Action::Return(r) => r,
            Action::Terminate => terminate_current_task(),
        }
    }
}
