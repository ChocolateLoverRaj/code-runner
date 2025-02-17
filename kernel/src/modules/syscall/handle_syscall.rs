use core::arch::{asm, naked_asm};

use conquer_once::noblock::OnceCell;
use x86_64::VirtAddr;

const TEMP_STACK_SIZE: usize = 0x10000;

#[repr(C, align(16))]
struct TempStack([u8; TEMP_STACK_SIZE]);

static mut TEMP_STACK: TempStack = TempStack([0; TEMP_STACK_SIZE]);

/// It's called "Rust" Syscall Handler to indicate that it can just be a normal Rust function, no messing with registers
/// Note that interrupts will be disabled when this function is called.
/// If you want, you can enable interrupts in this function, but you must make sure they are disabled before returning.
pub type RustSyscallHandler = extern "sysv64" fn(
    input0: u64,
    input1: u64,
    input2: u64,
    input3: u64,
    input4: u64,
    input5: u64,
    input6: u64,
    user_space_stack: VirtAddr,
) -> u64;

static RUST_HANDLER: OnceCell<RustSyscallHandler> = OnceCell::uninit();

// save the registers, handle the syscall and return to usermode
#[naked]
unsafe extern "sysv64" fn handle_syscall_wrapper() {
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
            // Convert `syscall`s `r10` input to `sysv64`s `rcx` input
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
        // The bug is caused by the same temp stack being written to by two syscall handlers at the same time. For the 2nd syscall handler, we need to set the `rsp` to the `rsp` of the currently paused syscall handler
        let temp_stack = {
            #[allow(static_mut_refs)]
            TEMP_STACK.0.as_mut_ptr().add(TEMP_STACK_SIZE)
        };
        asm!("\
            mov {old_stack}, rsp
            mov rsp, {temp_stack_base_plus_stack_size} // move our stack to the newly allocated one
            ",
            temp_stack_base_plus_stack_size = in(reg) temp_stack, old_stack = out(reg) old_stack
        );
    }

    // unwrap shouldn't panic cuz this handler will only be called after setting the handler
    let ret_val = RUST_HANDLER.try_get().unwrap()(
        arg0,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
        arg6,
        VirtAddr::from_ptr(old_stack),
    );

    unsafe {
        asm!("\
            mov rsp, {old_stack} // restore the old stack
            ",
            old_stack = in(reg) old_stack
        );
    }
    ret_val
}

pub fn get_syscall_handler(rust_handler: RustSyscallHandler) -> *const () {
    RUST_HANDLER.try_get_or_init(|| rust_handler).unwrap();
    handle_syscall_wrapper as *const ()
}
