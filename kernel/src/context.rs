use core::arch::asm;

use x86_64::{structures::gdt::SegmentSelector, PrivilegeLevel};

pub trait Context {
    /// # Safety
    /// Completely changes context
    unsafe fn restore(&self) -> !;

    fn privilege_level(&self) -> PrivilegeLevel;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FullContext {
    pub rbp: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl Context for FullContext {
    unsafe fn restore(&self) -> ! {
        unsafe {
            asm!("\
                mov rsp, {}
                pop rbp
                pop rax
                pop rbx
                pop rcx 
                pop rdx
                pop rsi
                pop rdi
                pop r8
                pop r9
                pop r10 
                pop r11
                pop r12
                pop r13
                pop r14
                pop r15
                iretq
                ",
                in(reg) self
            );
        }
        unreachable!()
    }

    fn privilege_level(&self) -> PrivilegeLevel {
        SegmentSelector(self.cs as u16).rpl()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// Context that you can use to exit from a syscall later. Everything else is saved by the user space program when calling syscall. The order doesn't really matter since the stack isn't modified by the CPU when entering and exiting a syscall handler.
pub struct SyscallContext {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub r11: u64,
    pub rcx: u64,
    pub rax: u64,
    pub rsp: u64,
}

impl Context for SyscallContext {
    unsafe fn restore(&self) -> ! {
        unsafe {
            asm!("\
                mov rsp, {}
                pop r15
                pop r14
                pop r13
                pop r12
                pop rbx
                pop rbp
                pop r11
                pop rcx
                pop rax
                pop rsp
                sysretq
                ",
                in(reg) self
            );
        }
        unreachable!()
    }

    fn privilege_level(&self) -> PrivilegeLevel {
        // Because restoring this context will immediately enter Ring3
        PrivilegeLevel::Ring3
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnyContext {
    Full(FullContext),
    Syscall(SyscallContext),
}

impl AnyContext {
    pub fn context(&self) -> &dyn Context {
        match self {
            AnyContext::Full(full_context) => full_context,
            AnyContext::Syscall(syscall_context) => syscall_context,
        }
    }
}
