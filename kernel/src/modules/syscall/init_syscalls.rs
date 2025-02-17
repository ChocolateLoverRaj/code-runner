use x86_64::{
    registers::{
        control::{Efer, EferFlags},
        model_specific::{LStar, Msr},
        rflags::RFlags,
    },
    VirtAddr,
};

use super::syscall_handler::SyscallHandler;

pub fn init_syscalls(syscall_handler: SyscallHandler) {
    // Enable syscall in IA32_EFER
    // https://shell-storm.org/x86doc/SYSCALL.html
    // https://wiki.osdev.org/CPU_Registers_x86-64#IA32_EFER
    unsafe {
        Efer::update(|flags| {
            *flags = flags.union(EferFlags::SYSTEM_CALL_EXTENSIONS);
        })
    };

    // clear Interrupt flag on syscall with AMD's MSR_FMASK register
    // This makes it so that interrupts are disabled during the syscall handler
    let mut msr_fmask = Msr::new(0xc0000084);
    unsafe { msr_fmask.write(RFlags::INTERRUPT_FLAG.bits()) };

    // write handler address to AMD's MSR_LSTAR register
    LStar::write(VirtAddr::from_ptr(syscall_handler.as_ptr()));
}
