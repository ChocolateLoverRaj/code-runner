use core::{arch::naked_asm, fmt::Debug, mem::offset_of};

use alloc::boxed::Box;

use crate::{
    cpu_local_data::{get_local, CpuLocalData},
    modules::syscall::syscall_handler::SyscallHandler,
};

// save the registers, handle the syscall and return to user mode
#[naked]
unsafe extern "sysv64" fn raw_syscall_handler() {
    unsafe {
        naked_asm!(
            "
            // Switch to temp stack
            swapgs

            // We need to disable interrupts until we do swapgs to make sure that the gs doesn't get swapped multiple times before here
            // sti

            // Save `rsp` to `THREAD_CONTROL_DATA.user_stack_pointer`
            mov gs:[{sp_offset}], rsp
            // Set `rsp` to `THREAD_CONTROL_DATA.kernel_stack_pointer`
            mov rsp, gs:[{ksp_offset}]


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

            // Call the function
            // Convert `syscall`s `r10` input to `sysv64`s `rcx` input
            mov rcx, r10
            // After the first 6 inputs, additional inputs go on the stack **in reverse order**. So we put `rax` on the stack
            push rsp // I added an extra input which is the kernel's stack pointer
            push rax // Move rax to the stack which is where additional inputs go in sysv64
            call {syscall_handler}

            // asm version of unreachable!() in rust
            ud2
            ",
            syscall_handler = sym syscall_handler,
            sp_offset = const offset_of!(CpuLocalData, user_stack_pointer),
            ksp_offset = const offset_of!(CpuLocalData, kernel_stack_pointer),
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

/// You must set `GS.Base` to the syscall handler's stack before the first syscall. You must do `swapgs` before entering user mode.
pub unsafe trait SyscallHandlerClosure: Send + Sync + Debug {
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
    ) -> !;
}

extern "sysv64" fn syscall_handler(
    input0: u64,
    input1: u64,
    input2: u64,
    input3: u64,
    input4: u64,
    input5: u64,
    input6: u64,
    rsp: u64,
) -> ! {
    let pushed_registers = unsafe { &mut *(rsp as *mut PushedRegisters) };
    (get_local()
        .unwrap()
        .syscall_handler_closure
        .try_get()
        .unwrap())
    .handle_syscall(
        input0,
        input1,
        input2,
        input3,
        input4,
        input5,
        input6,
        pushed_registers,
    )

    // let thread_control_data = unsafe { &THREAD_CONTROL_DATA };
    // log::info!(
    //     "Pushed registers: {:#?}. Stack info: {:#?}. Inputs: {:#?}",
    //     pushed_registers,
    //     thread_control_data,
    //     [input0, input1, input2, input3, input4, input5, input6]
    // );
    // let return_value = 333;
    // let s = SyscallContext {
    //     r15: pushed_registers.r15,
    //     r14: pushed_registers.r14,
    //     r13: pushed_registers.r13,
    //     r12: pushed_registers.r12,
    //     rbx: pushed_registers.rbx,
    //     rbp: pushed_registers.rbp,
    //     r11: pushed_registers.r11,
    //     rcx: pushed_registers.rcx,
    //     rax: return_value,
    //     rsp: unsafe { THREAD_CONTROL_DATA.user_stack_pointer.assume_init() } as u64,
    // };
    // unsafe { GS::swap() };
    // unsafe { s.restore() }
}

pub fn set_syscall_handler_closure(closure: Box<dyn SyscallHandlerClosure>) -> SyscallHandler {
    get_local()
        .unwrap()
        .syscall_handler_closure
        .try_init(closure)
        .unwrap();
    unsafe { SyscallHandler::new_unchecked(raw_syscall_handler) }
}
