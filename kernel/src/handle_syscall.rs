use core::arch::{asm, naked_asm};

const TEMP_STACK_SIZE: usize = 0x10000;

#[repr(C, align(16))]
struct TempStack([u8; TEMP_STACK_SIZE]);

static mut TEMP_STACK: TempStack = TempStack([0; TEMP_STACK_SIZE]);

// save the registers, handle the syscall and return to usermode
#[naked]
pub extern "C" fn handle_syscall_wrapper() {
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

            // Do the call 
            // Save the stack pointer (`rsp`) to `rbp`
            mov rbp, rsp
            // Adjust the input
            mov rcx, r10
            // After the first 6 inputs, additional inputs go on the stack. So we put `rax` on the stack
            push rax // Move rax to the stack which is where additional inputs go in sysv64
            call {handle_syscall_with_temp_stack}
            // restore `rsp` from `rbp`
            mov rsp, rbp 

            // restore callee-saved registers from the stack
            pop r15 
            pop r14
            pop r13
            pop r12
            pop rbx
            pop rbp 

            // restore registers from the stack for sysretq
            pop r11
            pop rcx

            // go back to user mode
            sysretq
            ",
            handle_syscall_with_temp_stack = sym handle_syscall_with_temp_stack
        );
    }
}

#[inline(always)]
extern "sysv64" fn handle_syscall_with_temp_stack(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    let old_stack: *const u8;
    unsafe {
        asm!("\
            nop
            mov {old_stack}, rsp
            nop
            mov rsp, {temp_stack_base_plus_stack_size} // move our stack to the newly allocated one
            nop
            sti // enable interrupts
            nop
            ",
            temp_stack_base_plus_stack_size = in(reg) TEMP_STACK.0.as_mut_ptr(), old_stack = out(reg) old_stack
        );
    }

    let ret_val = syscall_handler_inner(arg0, arg1, arg2, arg3, arg4, arg5, arg6);

    unsafe {
        asm!("\
            nop
            cli // disable interrupts while restoring the stack
            nop
            mov rsp, {old_stack} // restore the old stack
            nop
            ",
            old_stack = in(reg) old_stack
        );
    }
    ret_val
}

// Never inline to make sure that the local variables of this function are always in the kernel's stack and not the user space's stack
#[inline(never)]
extern "sysv64" fn syscall_handler_inner(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    log::info!(
        "Syscalled with args: 0x{:x} 0x{:x} 0x{:x} 0x{:x} 0x{:x} 0x{:x} 0x{:x}",
        arg0,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
        arg6
    );
    0xabcdef
}
