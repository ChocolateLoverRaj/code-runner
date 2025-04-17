use core::arch::naked_asm;

use common::syscall_uuids::{serialize_output, SyscallWaitUntilEvent};
use x86_64::{
    registers::{
        control::Cr3,
        model_specific::{GsBase, KernelGsBase},
        segmentation::GS,
    },
    structures::{gdt::SegmentSelector, idt::InterruptStackFrame},
    PrivilegeLevel,
};

use crate::{
    context::{AnyContext, FullContext, SyscallContext},
    cpu_local_data::get_local,
    init_idt_and_gdt::get_iopb,
    io_permission_bitmap::IoPermissionBitmap,
    io_ports_lock::IO_PORT_USAGE,
    tasks::{TaskState, TaskType, TASKS},
};

/// # Safety
/// This function should only be called as a keyboard interrupt handler, and not manually.
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

    let context = {
        let cpu_local_data = get_local().unwrap();
        unsafe {
            cpu_local_data
                .local_apic
                .try_get()
                .unwrap()
                .lock()
                .end_of_interrupt()
        };
        let mut tasks = TASKS.try_get().unwrap().lock();
        let keyboard_listener_task_id = tasks.keyboard_listener.as_ref().unwrap().task_id;
        let task = tasks
            .tasks
            .iter_mut()
            .find(|task| task.id == keyboard_listener_task_id)
            .unwrap();
        if let TaskState::WaitingUntilEvent(state) = &task.state {
            let s = SyscallContext::from_syscall_output(
                state,
                serialize_output::<SyscallWaitUntilEvent>(&()).unwrap(),
            );
            task.state = TaskState::Running;
            match &task.task_type {
                TaskType::User(data) => {
                    let cr3_flags = Cr3::read().1;
                    unsafe { Cr3::write(data.cr3, cr3_flags) };
                    let kernel_stack_pointer = data.kernel_stack.top().as_u64();
                    unsafe {
                        cpu_local_data
                            .kernel_stack_pointer
                            .get()
                            .write(kernel_stack_pointer)
                    };
                    cpu_local_data.task_data.lock().current_task = Some(task.id);
                    let mut iopb = get_iopb().lock();
                    **iopb = IoPermissionBitmap::new_deny_all();
                    let io_ports_usage = IO_PORT_USAGE.lock();
                    io_ports_usage
                        .iter()
                        .filter_map(|(port, id)| {
                            if id == &keyboard_listener_task_id {
                                Some(*port)
                            } else {
                                None
                            }
                        })
                        .for_each(|port| {
                            log::debug!("setting port allowed: {}", port);
                            iopb.set_port_allowed(port, true);
                        });
                }
            }
            tasks
                .keyboard_listener
                .as_mut()
                .unwrap()
                .pending_interrupt_received = false;
            log::debug!(
                "Restoring context with GS.Base: {:?}. Context: {:?}",
                KernelGsBase::read(),
                s
            );
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
