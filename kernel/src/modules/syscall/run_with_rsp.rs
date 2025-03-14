use core::{
    arch::{asm, naked_asm},
    ptr,
};

use alloc::boxed::Box;

/// Runs the closure with a new stack (changes the `rsp` register). Then changes the `rsp` register back to restore the old stack
pub fn run_with_rsp(new_rsp: u64, mut closure: impl FnOnce()) {
    let closure_ptr = ptr::from_mut(&mut closure) as *mut dyn FnOnce();
    unsafe {
        asm!(
            "
            call {}
            ",
            in(reg) run_with_rsp_asm as *const (),
            in("rdi") &closure_ptr,
            in("rsi") new_rsp,
        );
    }
}

#[naked]
unsafe extern "sysv64" fn run_with_rsp_asm() {
    unsafe {
        naked_asm!(
            "
            mov rbp, rsp
            mov rsp, rsi
            call {closure_caller}
            mov rsp, rbp
            ",
            closure_caller = sym closure_caller
        );
    }
}

extern "sysv64" fn closure_caller(closure: &*mut dyn FnOnce()) {
    unsafe {
        let temp_box = Box::from_raw(*closure);
        temp_box();
    };
}
