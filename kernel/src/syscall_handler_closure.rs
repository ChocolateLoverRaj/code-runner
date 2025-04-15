use core::fmt::Debug;

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use common::syscall_uuids::{
    from_input_without_uuid, get_uuid, serialize_output, Syscall, SyscallExists, SyscallExit,
    SyscallListenForKeyboardInterrupts, SyscallListenForKeyboardInterruptsOutputError, SyscallLog,
    SyscallTakeIoPort, SyscallTakeIoPortOutputError, SyscallTest, SyscallWaitUntilEvent,
};
use spinning_top::Spinlock;
use uuid::Uuid;
use x2apic::ioapic::{IrqMode, RedirectionTableEntry};
use x86_64::registers::segmentation::GS;

use crate::{
    context::{Context, SyscallContext},
    cpu_local_data::get_local,
    init_idt_and_gdt::{get_iobp, MAPPED_APICS},
    pic8259_interrupts::Pic8259Interrupts,
    run_tasks::run_tasks,
    syscall_handler::{PushedRegisters, SyscallHandlerClosure},
    tasks::{KeyboardEventListener, TaskState, TaskType, TASKS},
    terminate_current_task::terminate_current_task,
};

pub trait Includes<K> {
    fn contains_key(&self, key: &K) -> bool;
}

impl<K: Ord, V> Includes<K> for BTreeMap<K, V> {
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

pub type SyscallHandler =
    dyn Fn(&[u64; 5], &mut PushedRegisters, &dyn Includes<Uuid>) -> [u64; 7] + Send + Sync;

fn make_syscall_handler<T: Syscall>(
    f: impl Fn(T::Input, &PushedRegisters, &dyn Includes<Uuid>) -> T::Output + Send + Sync,
) -> impl Fn(&[u64; 5], &mut PushedRegisters, &dyn Includes<Uuid>) -> [u64; 7] + Send + Sync {
    move |input, pushed_registers, syscalls| {
        let input = from_input_without_uuid::<T>(input).unwrap();
        let output = f(input, pushed_registers, syscalls);
        serialize_output::<T>(&output).unwrap()
    }
}

#[derive(Default)]
struct SyscallHandlers {
    handlers: BTreeMap<Uuid, Box<SyscallHandler>>,
}
impl SyscallHandlers {
    fn insert<T: Syscall + 'static>(
        &mut self,
        handler: impl Fn(T::Input, &PushedRegisters, &dyn Includes<Uuid>) -> T::Output
            + Send
            + Sync
            + 'static,
    ) {
        self.handlers
            .insert(T::UUID, Box::new(make_syscall_handler::<T>(handler)));
    }
}

impl Debug for SyscallHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(core::any::type_name::<Self>())
            .field("syscalls", &self.handlers.keys())
            .finish()
    }
}

// Safety: We do what it tells us to do
unsafe impl SyscallHandlerClosure for SyscallHandlers {
    fn handle_syscall(
        &self,
        input0: u64,
        input1: u64,
        input2: u64,
        input3: u64,
        input4: u64,
        input5: u64,
        input6: u64,
        pushed_registers: &mut PushedRegisters,
    ) -> ! {
        let (uuid, input) =
            get_uuid([input0, input1, input2, input3, input4, input5, input6]).unwrap();
        match self.handlers.get(&uuid) {
            None => {
                log::warn!("Invalid syscall {:?}. Terminating process.", uuid);
                terminate_current_task()
            }
            Some(syscall_handler) => {
                let output = syscall_handler(&input, pushed_registers, &self.handlers);
                let s = SyscallContext::from_syscall_output(pushed_registers, output);
                unsafe { GS::swap() };
                unsafe { s.restore() }
            }
        }
    }
}

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
    // TODO: We are going to run into lock problems later
    static IO_PORT_USAGE: Spinlock<BTreeMap<u16, usize>> = Spinlock::new(BTreeMap::new());
    syscall_handlers.insert::<SyscallTakeIoPort>(|port, _, _| {
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
                    let mut iopb = get_iobp().lock();
                    if iopb.contains_port_permission(port) {
                        if task_data.permissions.ports.contains(&port) {
                            let mut m = IO_PORT_USAGE.lock();
                            if !m.contains_key(&port) {
                                m.insert(port, current_task_id);
                                iopb.set_port_allowed(port, true);
                                Action::Return(Ok(()))
                            } else {
                                Action::Return(Err(SyscallTakeIoPortOutputError::InUse))
                            }
                        } else {
                            log::warn!("Process {} tried to access a io port it doesn't have permission for. Terminating.", current_task_id);
                            Action::Terminate
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
    });
    syscall_handlers.insert::<SyscallListenForKeyboardInterrupts>(|_, _, _| {
        enum Action {
            Return(<SyscallListenForKeyboardInterrupts as Syscall>::Output),
            Terminate,
        }
        let action = {
            let mut tasks = TASKS.try_get().unwrap().lock();
            if tasks.keyboard_listener.is_some() {
            let current_task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
            let current_task = tasks
                .tasks
                .iter_mut()
                .find(|task| task.id == current_task_id)
                .unwrap();
            match &current_task.task_type {
                TaskType::User(data) => {
                    if data.permissions.keyboard_interrupts {
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
                        log::warn!("Task {} tried to listen for keyboard interrupts when it is not allowed to. Terminating.", current_task_id);
                        Action::Terminate
                    }
                }
            }
            } else {
Action::Return(Err(SyscallListenForKeyboardInterruptsOutputError::InUse))
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
