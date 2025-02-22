use core::{
    arch::naked_asm,
    ops::{Deref, DerefMut},
};

use alloc::sync::Arc;
use common::syscall_start_recording_keyboard::{
    FullQueueBehavior, SyscallStartRecordingKeyboardInput,
};
use conquer_once::noblock::OnceCell;
use crossbeam_queue::ArrayQueue;
use spin::{Mutex, RwLock, RwLockReadGuard};
use x2apic::{
    ioapic::{IoApic, RedirectionTableEntry},
    lapic::LocalApic,
};
use x86_64::{
    instructions::port::Port,
    structures::idt::{self, HandlerFunc, InterruptStackFrame},
    VirtAddr,
};

use crate::{
    context::{AnyContext, FullContext},
    enter_user_mode::enter_user_mode,
    modules::idt::IdtBuilder,
    pic8259_interrupts::Pic8259Interrupts,
    user_space_state::State,
};

static LOCAL_APIC: OnceCell<&'static OnceCell<Mutex<LocalApic>>> = OnceCell::uninit();

struct RecordingKeyboard {
    full_queue_behavior: FullQueueBehavior,
    queue: ArrayQueue<u8>,
}

static SCAN_CODE_QUEUE: RwLock<Option<RecordingKeyboard>> = RwLock::new(None);
pub static USER_SPACE_INTERRUPT_HANDLER: Mutex<Option<VirtAddr>> = Mutex::new(None);
static STATE: OnceCell<Arc<Mutex<State>>> = OnceCell::uninit();

#[naked]
unsafe extern "sysv64" fn context_switching_keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
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
            context_switch = sym context_switching_keyboard_interrupt_handler_rust
        );
    };
}

unsafe extern "sysv64" fn context_switching_keyboard_interrupt_handler_rust(
    context: *const FullContext,
) -> ! {
    let context = unsafe { *context };
    {
        // log::info!("State: {:#x?}", STATE.try_get().unwrap().lock().deref());
    }
    // log::info!("Context: {:#x?}", context);
    // Make sure to drop all locks before exiting
    #[derive(Debug)]
    enum JmpTo {
        UserMode(VirtAddr, VirtAddr),
        RestoreContext(AnyContext),
    }
    let jmp_to = {
        let mut port = Port::new(0x60);
        let scan_code: u8 = unsafe { port.read() };
        if let Some(RecordingKeyboard {
            full_queue_behavior,
            queue,
        }) = SCAN_CODE_QUEUE.read().deref()
        {
            match full_queue_behavior {
                FullQueueBehavior::DropNewest => {
                    let _ = queue.push(scan_code);
                }
                FullQueueBehavior::DropOldest => {
                    queue.force_push(scan_code);
                }
            }
        };
        let mut local_apic = LOCAL_APIC.try_get().unwrap().try_get().unwrap().lock();
        unsafe { local_apic.end_of_interrupt() };

        let mut user_space_state = STATE.try_get().unwrap().lock();
        let user_space_state = user_space_state.as_mut().unwrap();
        let user_space_interrupt_handler = USER_SPACE_INTERRUPT_HANDLER.lock();
        let user_space_interrupt_handler = user_space_interrupt_handler.as_ref();
        // This interrupt interrupted one of two things
        // - A hlt loop from a syscall handler. We just enter the handler then. If no handler exists then we restore context.
        // - The user space process. We save this context and enter the handler.
        match user_space_state.stack_of_saved_contexts.last() {
            None => {
                if let Some(user_space_interrupt_handler) = user_space_interrupt_handler {
                    if !user_space_state.in_keyboard_interrupt_handler
                        && user_space_state.interrupts_enabled
                    {
                        user_space_state
                            .stack_of_saved_contexts
                            .push_within_capacity(AnyContext::Full(context))
                            .unwrap();
                        // Continue the stack
                        let interrupt_handler_stack_end = VirtAddr::new(context.rsp);
                        user_space_state.in_keyboard_interrupt_handler = true;
                        JmpTo::UserMode(*user_space_interrupt_handler, interrupt_handler_stack_end)
                    } else {
                        user_space_state.keyboard_interrupt_queued = true;
                        JmpTo::RestoreContext(AnyContext::Full(context))
                    }
                } else {
                    // Just exit this interrupt handler
                    JmpTo::RestoreContext(AnyContext::Full(context))
                }
            }
            Some(AnyContext::Syscall(syscall_context)) => {
                if let Some(user_space_interrupt_handler) = user_space_interrupt_handler {
                    if !user_space_state.interrupts_enabled {
                        unreachable!("You can't disable interrupts and then wait for an interrupt to happen, because then the function would never be called. The syscall for waiting for an interrupt to happen should've returned an error.");
                    }
                    if !user_space_state.in_keyboard_interrupt_handler {
                        // Continue the stack
                        let interrupt_handler_stack_end = VirtAddr::new(syscall_context.rsp);
                        user_space_state.in_keyboard_interrupt_handler = true;
                        JmpTo::UserMode(*user_space_interrupt_handler, interrupt_handler_stack_end)
                    } else {
                        // Queue up
                        user_space_state.keyboard_interrupt_queued = true;
                        JmpTo::RestoreContext(AnyContext::Full(context))
                    }
                } else {
                    // sysretq
                    JmpTo::RestoreContext(AnyContext::Syscall(*syscall_context))
                }
            }
            Some(AnyContext::Full(_full_context)) => {
                // This means that we have an interrupt in the middle of a user space interrupt handler
                // Sine we only have one interrupt handler (keyboard), we just queue another one
                if !user_space_state.in_keyboard_interrupt_handler {
                    unreachable!("In a user space interrupt handler that is not a keyboard interrupt handler. Impossible.");
                } else {
                    user_space_state.keyboard_interrupt_queued = true;
                    JmpTo::RestoreContext(AnyContext::Full(context))
                }
            }
        }
    };
    // log::info!("jmp_to: {:#?}", jmp_to);
    match jmp_to {
        JmpTo::UserMode(user_space_interrupt_handler, interrupt_handler_stack_end) => {
            unsafe { enter_user_mode(user_space_interrupt_handler, interrupt_handler_stack_end) };
        }
        JmpTo::RestoreContext(context) => {
            unsafe { context.context().restore() };
        }
    }
}

unsafe fn enable_interrupts(io_apic: &mut IoApic) {
    unsafe { io_apic.enable_irq(Pic8259Interrupts::Keyboard.into()) }
}

unsafe fn disable_interrupts(io_apic: &mut IoApic) {
    unsafe { io_apic.disable_irq(Pic8259Interrupts::Keyboard.into()) }
}

pub struct CoolKeyboardBuilder {
    interrupt_index: u8,
}

impl CoolKeyboardBuilder {
    pub fn set_interrupt(
        idt_builder: &mut IdtBuilder,
        local_apic: &'static OnceCell<Mutex<LocalApic>>,
    ) -> Option<Self> {
        LOCAL_APIC.try_init_once(|| local_apic).unwrap();
        let interrupt_index = idt_builder.set_flexible_entry({
            let mut entry = idt::Entry::<HandlerFunc>::missing();
            entry.set_handler_fn({
                unsafe {
                    core::mem::transmute::<*const (), HandlerFunc>(
                        context_switching_keyboard_interrupt_handler as *const _,
                    )
                }
            });
            entry
        })?;
        Some(Self { interrupt_index })
    }

    pub fn configure_io_apic(
        &'static self,
        io_apic: Arc<Mutex<IoApic>>,
        state: Arc<Mutex<State>>,
    ) -> CoolKeyboard {
        {
            let mut io_apic = io_apic.lock();
            unsafe {
                io_apic.set_table_entry(Pic8259Interrupts::Keyboard.into(), {
                    let mut entry = RedirectionTableEntry::default();
                    entry.set_vector(self.interrupt_index);
                    entry
                })
            };
            STATE.try_init_once(|| state).unwrap();
        }
        CoolKeyboard { io_apic }
    }
}

#[derive(Debug, Clone)]
pub struct CoolKeyboard {
    io_apic: Arc<Mutex<IoApic>>,
}

impl CoolKeyboard {
    pub fn enable(&self, settings: SyscallStartRecordingKeyboardInput) {
        *SCAN_CODE_QUEUE.write() = Some(RecordingKeyboard {
            full_queue_behavior: settings.behavior_on_full_queue,
            queue: ArrayQueue::new(settings.queue_size as usize),
        });
        unsafe { enable_interrupts(self.io_apic.lock().deref_mut()) };
    }

    pub fn disable(&self) {
        unsafe { disable_interrupts(self.io_apic.lock().deref_mut()) };
    }

    pub fn queue(&self) -> QueueGuard {
        QueueGuard {
            guard: SCAN_CODE_QUEUE.read(),
        }
    }

    pub fn set_user_space_interrupt(&self, user_space_interrupt: Option<VirtAddr>) {
        *USER_SPACE_INTERRUPT_HANDLER.lock() = user_space_interrupt
    }
}

pub struct QueueGuard<'a> {
    guard: RwLockReadGuard<'a, Option<RecordingKeyboard>>,
}

impl QueueGuard<'_> {
    pub fn queue(&self) -> Option<&ArrayQueue<u8>> {
        self.guard
            .as_ref()
            .map(|recording_keyboard| &recording_keyboard.queue)
    }
}
