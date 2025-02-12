use conquer_once::noblock::OnceCell;
use spin::Mutex;
use x2apic::lapic::LocalApic;
use x86_64::structures::idt::{HandlerFunc, InterruptStackFrame};

use core::arch::naked_asm;

use crate::{context::Context, restore_context::restore_context};

static LOCAL_APIC: OnceCell<&'static OnceCell<Mutex<LocalApic>>> = OnceCell::uninit();

/// This is private so that the getter must be initialized before using
#[naked]
unsafe extern "sysv64" fn context_switching_logging_timer_interrupt_handler(
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
            sub rsp, 0x400
            jmp {context_switch}
            ", 
            context_switch = sym a
        );
    };
}

unsafe extern "sysv64" fn a(context: *const Context) {
    let context = unsafe { (*context).clone() };
    {
        let mut local_apic = LOCAL_APIC.try_get().unwrap().try_get().unwrap().lock();
        let current = unsafe { local_apic.timer_current() };
        // Don't log too often cuz then we would always only be executing this interrupt handler and no other code would get a chance to run.
        static mut COUNTER: u64 = 0;
        if unsafe { COUNTER } == 0 {
            log::info!("Timer interrupt (1 out of every 100 logged): {}", current);
        }
        unsafe {
            COUNTER += 1;
        }
        if unsafe { COUNTER } == 100 {
            unsafe { COUNTER = 0 };
        }

        unsafe { local_apic.end_of_interrupt() };
    }
    unsafe { restore_context(&context) };
}

pub fn get_context_switching_logging_timer_interrupt_handler(
    local_apic: &'static OnceCell<Mutex<LocalApic>>,
) -> HandlerFunc {
    LOCAL_APIC.try_init_once(|| local_apic).unwrap();
    unsafe { core::mem::transmute(context_switching_logging_timer_interrupt_handler as *const ()) }
}
