use core::arch::naked_asm;

use common::syscall_uuids::{serialize_output, SyscallWaitUntilEvent};
use x86_64::{
    registers::{
        model_specific::{GsBase, KernelGsBase},
        segmentation::GS,
    },
    structures::{gdt::SegmentSelector, idt::InterruptStackFrame},
    PrivilegeLevel,
};

use crate::{
    context::{AnyContext, FullContext, SyscallContext},
    cpu_local_data::get_local,
    tasks::{TaskState, TASKS},
};

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
            log::debug!("Swapped GS");
            true
        } else {
            log::debug!("Didn't swap GS");
            false
        };

    {
        unsafe {
            get_local()
                .unwrap()
                .local_apic
                .try_get()
                .unwrap()
                .lock()
                .end_of_interrupt()
        };
    }
    let context = {
        let mut tasks = TASKS.try_get().unwrap().lock();
        let keyboard_listener_task_id = tasks.keyboard_listener.as_ref().unwrap().task_id;
        let task = tasks
            .tasks
            .iter_mut()
            .find(|task| task.id == keyboard_listener_task_id)
            .unwrap();
        if let TaskState::WaitingUntilEvent(pushed_registers) = &task.state {
            let s = SyscallContext::from_syscall_output(
                pushed_registers,
                serialize_output::<SyscallWaitUntilEvent>(&()).unwrap(),
            );
            task.state = TaskState::Running;
            tasks
                .keyboard_listener
                .as_mut()
                .unwrap()
                .pending_interrupt_received = false;
            log::debug!("Restoring context with GS.Base: {:?}", KernelGsBase::read());
            // Make sure that when we enter user mode it is with user mode's gs base
            unsafe { GS::swap() };
            AnyContext::Syscall(s)
        } else {
            tasks
                .keyboard_listener
                .as_mut()
                .unwrap()
                .pending_interrupt_received = true;
            log::debug!("Restoring full context. GS.Base is: {:?}", GsBase::read());
            // Make sure that GS.Base is not different than what it was before this interrupt handler
            if swapped_gs {
                unsafe { GS::swap() };
            }
            AnyContext::Full(*context)
        }
    };
    unsafe { context.context().restore() }
}
