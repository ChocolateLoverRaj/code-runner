use core::{
    alloc::Layout,
    arch::{asm, naked_asm},
};

use alloc::alloc::{alloc, dealloc};

// save the registers, handle the syscall and return to usermode
#[naked]
pub extern "C" fn handle_syscall_wrapper() {
    unsafe {
        naked_asm!("\
        push rcx // backup registers for sysretq
        push r11
        push rbp // save callee-saved registers
        push rbx
        push r12
        push r13
        push r14
        push r15
        mov rbp, rsp // save rsp
        mov rcx, r10 // move syscall arg4 to rcx which is the fourth argument register in sysv64
        push rax // Move rax to the stack which is where additional inputs go in sysv64
        call {syscall_alloc_stack} // call the handler with the syscall number in r8
        mov rsp, rbp // restore rsp from rbp
        pop r15 // restore callee-saved registers
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp // restore stack and registers for sysretq
        pop r11
        pop rcx
        sysretq // back to userland",
        syscall_alloc_stack = sym syscall_alloc_stack);
    }
}

// allocate a temp stack and call the syscall handler
unsafe extern "sysv64" fn syscall_alloc_stack(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    const TEMP_STACK_SIZE: usize = 0x10000;
    let layout = Layout::from_size_align(TEMP_STACK_SIZE, 16).unwrap();
    // FIXME: Maybe don't put the allocator's stack on the userspace stack
    let stack_ptr = unsafe { alloc(layout) };
    let retval = handle_syscall_with_temp_stack(
        arg0,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
        arg6,
        stack_ptr.wrapping_add(TEMP_STACK_SIZE),
    );
    unsafe { dealloc(stack_ptr, layout) };
    retval
}

#[inline(never)]
extern "sysv64" fn handle_syscall_with_temp_stack(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
    temp_stack_base_plus_stack_size: *const u8,
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
        nop",
        temp_stack_base_plus_stack_size = in(reg) temp_stack_base_plus_stack_size, old_stack = out(reg) old_stack);
    }

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

    let retval: u64 = 0xabcdef;
    unsafe {
        asm!("\
            nop
        cli // disable interrupts while restoring the stack
        nop
        mov rsp, {old_stack} // restore the old stack
        nop
        ",
        old_stack = in(reg) old_stack);
    }
    retval
}
