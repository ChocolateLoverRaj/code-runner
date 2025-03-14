use core::{arch::naked_asm, mem};

use alloc::boxed::Box;
use conquer_once::noblock::OnceCell;

use super::syscall_handler::SyscallHandler;

// save the registers, handle the syscall and return to user mode
#[naked]
unsafe extern "sysv64" fn raw_syscall_handler() {
    unsafe {
        naked_asm!("\
            // backup registers for sysretq
            push rcx
            push r11

            // save callee-saved registers on the stack
            push rbp
            push rbx
            push r12
            push r13
            push r14
            push r15

            // Switch to temp stack
            mov rbp, rsp

            // Call the function
            // Convert `syscall`s `r10` input to `sysv64`s `rcx` input
            mov rcx, r10
            // After the first 6 inputs, additional inputs go on the stack **in reverse order**. So we put `rax` on the stack
            push rbp // I added an extra input which is the user space stack pointer
            push rax // Move rax to the stack which is where additional inputs go in sysv64
            call {syscall_handler}

            // asm version of unreachable!() un rust
            ud2
            ",
            syscall_handler = sym syscall_handler,
        );
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PushedRegisters {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub r11: u64,
    pub rcx: u64,
}

// pub trait SyscallHandlerClosureTrait: Sync + Send {
//     /// This function runs on the user space stack. You must switch stacks yourself to avoid leaking data to user space through the stack. Also you must handle stack overflows, because the user space function could have very little stack space left.
//     fn call(
//         &self,
//         input0: u64,
//         input1: u64,
//         input2: u64,
//         input3: u64,
//         input4: u64,
//         input5: u64,
//         input6: u64,
//         user_space_rsp_to_restore: u64,
//         pushed_registers: &PushedRegisters,
//     ) -> !;
// }

// impl<T: (Fn(u64, u64, u64, u64, u64, u64, u64, u64, &PushedRegisters) -> !) + Send + Sync>
//     SyscallHandlerClosureTrait for T
// {
//     fn call(
//         &self,
//         input0: u64,
//         input1: u64,
//         input2: u64,
//         input3: u64,
//         input4: u64,
//         input5: u64,
//         input6: u64,
//         user_space_rsp_to_restore: u64,
//         pushed_registers: &PushedRegisters,
//     ) -> ! {
//         self.call((
//             input0,
//             input1,
//             input2,
//             input3,
//             input4,
//             input5,
//             input6,
//             user_space_rsp_to_restore,
//             pushed_registers,
//         ))
//     }
// }

pub type SyscallHandlerClosure =
    dyn Fn(u64, u64, u64, u64, u64, u64, u64, u64, &PushedRegisters) -> ! + Sync + Send;

static CLOSURE: OnceCell<Box<SyscallHandlerClosure>> = OnceCell::uninit();

extern "sysv64" fn syscall_handler(
    input0: u64,
    input1: u64,
    input2: u64,
    input3: u64,
    input4: u64,
    input5: u64,
    input6: u64,
    user_space_stack_pointer: u64,
) -> ! {
    CLOSURE.try_get().unwrap()(
        input0,
        input1,
        input2,
        input3,
        input4,
        input5,
        input6,
        user_space_stack_pointer + size_of::<PushedRegisters>() as u64,
        unsafe { mem::transmute(user_space_stack_pointer as *const PushedRegisters) },
    );
}

pub fn set_syscall_handler_closure(closure: Box<SyscallHandlerClosure>) -> SyscallHandler {
    CLOSURE.try_init_once(|| closure).unwrap();
    unsafe { SyscallHandler::new_unchecked(raw_syscall_handler) }
}
