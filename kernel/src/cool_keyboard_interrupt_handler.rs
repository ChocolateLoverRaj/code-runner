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
    context::Context, enter_user_mode::enter_user_mode, modules::idt::IdtBuilder,
    pic8259_interrupts::Pic8259Interrupts, restore_context::restore_context,
};

static LOCAL_APIC: OnceCell<&'static OnceCell<Mutex<LocalApic>>> = OnceCell::uninit();

struct RecordingKeyboard {
    full_queue_behavior: FullQueueBehavior,
    queue: ArrayQueue<u8>,
}

static SCANCODE_QUEUE: RwLock<Option<RecordingKeyboard>> = RwLock::new(None);
static USER_SPACE_INTERRUPT_HANDLER: Mutex<Option<VirtAddr>> = Mutex::new(None);
static CONTEXT_TO_GO_BACK_TO: OnceCell<Arc<Mutex<Option<Context>>>> = OnceCell::uninit();

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
            // mov rsi, [rsp + 0xA0] // 2nd arg: interrupt stack frame
            
            sub rsp, 0x400
            jmp {context_switch}
            ", 
            context_switch = sym context_switching_keyboard_interrupt_handler_rust
        );
    };
}

unsafe extern "sysv64" fn context_switching_keyboard_interrupt_handler_rust(
    context: *const Context,
    // interrupt_stack_frame: *const InterruptStackFrame,
) {
    let context = unsafe { (*context).clone() };
    // let stack_frame = unsafe { (*interrupt_stack_frame).clone() };
    log::info!("Context: {:#x?}", context);
    // Make sure to drop all locks before exiting
    #[derive(Debug)]
    enum JmpTo {
        UserMode(VirtAddr, VirtAddr),
        RestoreContext(Context),
    }
    let jmp_to = {
        let mut port = Port::new(0x60);
        let scan_code: u8 = unsafe { port.read() };
        if let Some(RecordingKeyboard {
            full_queue_behavior,
            queue,
        }) = SCANCODE_QUEUE.read().deref()
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
        match USER_SPACE_INTERRUPT_HANDLER.lock().as_ref() {
            Some(user_space_interrupt_handler) => {
                *CONTEXT_TO_GO_BACK_TO.try_get().unwrap().lock() = Some(context);
                // FIXME: Make sure that the context we have is actually the context for the same user space process, so that it has the correct stack and rsp
                let interrupt_handler_stack_end = VirtAddr::new(context.rsp);
                JmpTo::UserMode(*user_space_interrupt_handler, interrupt_handler_stack_end)
            }
            None => JmpTo::RestoreContext(context),
        }
    };
    log::info!("jmp_to: {:#?}", jmp_to);
    match jmp_to {
        JmpTo::UserMode(user_space_interrupt_handler, interrupt_handler_stack_end) => {
            unsafe { enter_user_mode(user_space_interrupt_handler, interrupt_handler_stack_end) };
        }
        JmpTo::RestoreContext(context) => {
            unsafe { restore_context(&context) };
        }
    }
}

// extern "x86-interrupt" fn cool_keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
//     let mut port = Port::new(0x60);
//     let scan_code: u8 = unsafe { port.read() };
//     if let Some(RecordingKeyboard {
//         full_queue_behavior,
//         queue,
//     }) = SCANCODE_QUEUE.read().deref()
//     {
//         match full_queue_behavior {
//             FullQueueBehavior::DropNewest => {
//                 let _ = queue.push(scan_code);
//             }
//             FullQueueBehavior::DropOldest => {
//                 queue.force_push(scan_code);
//             }
//         }
//     };
//     let mut local_apic = LOCAL_APIC.try_get().unwrap().try_get().unwrap().lock();
//     unsafe { local_apic.end_of_interrupt() };
// }

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
        context_to_go_back_to: Arc<Mutex<Option<Context>>>,
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
            CONTEXT_TO_GO_BACK_TO
                .try_init_once(|| context_to_go_back_to)
                .unwrap();
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
        *SCANCODE_QUEUE.write() = Some(RecordingKeyboard {
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
            guard: SCANCODE_QUEUE.read(),
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
