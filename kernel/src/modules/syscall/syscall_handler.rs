pub type RawSyscallHandler = unsafe extern "sysv64" fn();

/// A wrapper type for syscall handlers, to reduce the chances of setting the syscall handler to the wrong function
pub struct SyscallHandler(RawSyscallHandler);

impl SyscallHandler {
    /// # Safety
    /// You need to have a proper syscall handler, which includes:
    /// - Not messing up the stack
    /// - Not messing up callee-saved registers
    /// - Not exposing kernel information to the user space process (which probably means switching stacks)
    pub const unsafe fn new_unchecked(raw_syscall_handler: RawSyscallHandler) -> Self {
        Self(raw_syscall_handler)
    }

    pub const fn as_ptr(&self) -> *const () {
        self.0 as *const ()
    }
}
