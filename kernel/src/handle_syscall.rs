use core::{
    alloc::Layout,
    arch::{asm, naked_asm},
    cell::UnsafeCell,
    fmt::Write,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::alloc::{alloc, dealloc};
use bootloader_x86_64_common::serial::SerialPort;
use conquer_once::noblock::OnceCell;
use spin::Mutex;
use x86_64::{
    instructions,
    structures::paging::{PageSize, Size4KiB},
};

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
        sub rsp, 0x400 // make some room in the stack
        mov rcx, r10 // move fourth syscall arg to rcx which is the fourth argument register in sysv64
        mov r8, rax // move syscall number to the 5th argument register
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

// static SERIAL_PORT: OnceCell<Mutex<SerialPort>> = OnceCell::uninit();
pub static TEST: AtomicBool = AtomicBool::new(false);

// allocate a temp stack and call the syscall handler
//  extern "sysv64"
unsafe extern "sysv64" fn syscall_alloc_stack(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    syscall: u64,
) -> u64 {
    static mut syscall_stack: [u8; 0x10000] = [0; 0x10000];
    // let syscall_stack: Vec<u8> = Vec::with_capacity(0x10000);
    // let layout = Layout::from_size_align(0x10000, Size4KiB::SIZE as usize).unwrap();
    // let stack_ptr = alloc(layout);
    let stack_ptr = syscall_stack.as_ptr();
    let retval = handle_syscall_with_temp_stack(arg0, arg1, arg2, arg3, syscall, stack_ptr);
    // dealloc(stack_ptr, layout);
    // drop(syscall_stack); // we can now drop the syscall temp stack
    return retval;
}

#[inline(never)]
extern "sysv64" fn handle_syscall_with_temp_stack(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    syscall: u64,
    temp_stack: *const u8,
) -> u64 {
    let old_stack: *const u8;
    unsafe {
        asm!("\
        mov {old_stack}, rsp
        mov rsp, {temp_stack} // move our stack to the newly allocated one
        // sti // enable interrupts",
        temp_stack = in(reg) temp_stack, old_stack = out(reg) old_stack);
    }

    log::warn!("Syscalled with args: {} {} {} {}", arg0, arg1, arg2, arg3);

    TEST.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);

    let mut s = unsafe { SerialPort::init() };
    writeln!(
        s,
        "Syscalled with args: {} {} {} {}",
        arg0, arg1, arg2, arg3
    );

    let retval: u64 = 4;
    unsafe {
        asm!("\
        // cli // disable interrupts while restoring the stack
        mov rsp, {old_stack} // restore the old stack
        ",
        old_stack = in(reg) old_stack);
    }
    retval
}
