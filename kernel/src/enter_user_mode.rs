use core::arch::asm;
use x86_64::{
    instructions::tlb,
    registers::segmentation::{Segment, DS},
    PrivilegeLevel, VirtAddr,
};

use crate::modules::gdt::Gdt;

/// # Safety
/// Jumps to an unchecked address with an unchecked stack.
pub unsafe fn enter_user_mode(gdt: &Gdt, code: VirtAddr, stack_end: VirtAddr) {
    let cs_idx = {
        let mut code_selector = gdt.user_code_selector;
        code_selector.set_rpl(PrivilegeLevel::Ring3);
        code_selector.0
    };
    let ds_idx = {
        let mut data_selector = gdt.user_data_selector;
        data_selector.set_rpl(PrivilegeLevel::Ring3);
        unsafe { DS::set_reg(data_selector) };
        data_selector.0
    };
    tlb::flush_all();
    log::info!("Last message before entering user mode");
    unsafe {
        asm!("\
    push rax   // stack segment
    push rsi   // rsp
    push 0x200 // rflags (only interrupt bit set)
    push rdx   // code segment
    push rdi   // ret to virtual addr
    iretq",
    in("rdi") code.as_u64(), in("rsi") stack_end.as_u64(), in("dx") cs_idx, in("ax") ds_idx);
    }
}
