use x86_64::{registers::model_specific::Msr, VirtAddr};

pub unsafe fn init_syscalls(syscall_handler: VirtAddr) {
    // Enable syscall in IA32_EFER
    // https://shell-storm.org/x86doc/SYSCALL.html
    // https://wiki.osdev.org/CPU_Registers_x86-64#IA32_EFER
    let mut ia32_efer = Msr::new(0xC0000080);
    let mut value = unsafe { ia32_efer.read() };
    value |= 0b1;
    unsafe { ia32_efer.write(value) };

    // clear Interrupt flag on syscall with AMD's MSR_FMASK register
    // This makes it so that interrupts are disabled during the syscall handler
    let mut msr_fmask = Msr::new(0xc0000084);
    unsafe { msr_fmask.write(0x200) };

    // write handler address to AMD's MSR_LSTAR register
    let mut msr_lstar: Msr = Msr::new(0xc0000082); // MSR_LSTAR
    unsafe { msr_lstar.write(syscall_handler.as_u64()) };

    let mut msr_star = Msr::new(0xc0000081); // MSR_STAR
    unsafe { msr_star.write(0x230008) };
}
