use core::{arch::asm, ptr};

/// Runs the closure with a new stack (changes the `rsp` register). Then changes the `rsp` register back to restore the old stack
pub fn run_with_rsp(new_rsp: u64, closure: impl FnOnce()) {
    // Basically convert `FnOnce` into `FnMut`, panicking if it is called more than once (it won't be)
    let mut fn_mut_closure = {
        let mut closure = Some(closure);
        move || closure.take().unwrap()()
    };
    let closure_ptr = ptr::from_mut(&mut fn_mut_closure) as *mut dyn FnMut();
    unsafe {
        // asm!(
        //     "
        //     call {}
        //     ",
        //     in(reg) run_with_rsp_asm as *const (),
        //     in("rdi") &closure_ptr,
        //     in("rsi") new_rsp,
        // );
        asm!(
            "
            mov rbp, rsp
            mov rsp, rsi
            call {closure_caller}
            mov rsp, rbp
            ",
            in("rdi") &closure_ptr,
            in("rsi") new_rsp,
            closure_caller = sym closure_caller
        );
    }
}

/// The closure must be a `FnMut` and not a `FnOnce` because `FnOnce` requires `self` as an argument, and we don't know the size of `self`.
extern "sysv64" fn closure_caller(closure: &*mut dyn FnMut()) {
    unsafe { (**closure)() };
}
