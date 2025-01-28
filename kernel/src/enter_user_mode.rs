use core::arch::asm;
use x86_64::{
    instructions::tlb,
    registers::segmentation::{Segment, DS},
    PrivilegeLevel, VirtAddr,
};

use crate::modules::gdt::Gdt;

pub unsafe fn enter_user_mode(gdt: &Gdt, code: VirtAddr, stack_end: VirtAddr) {
    let cs_idx = {
        let mut code_selector = gdt.user_code_selector.clone();
        code_selector.set_rpl(PrivilegeLevel::Ring3);
        code_selector.0
    };
    let ds_idx = {
        let mut data_selector = gdt.user_data_selector.clone();
        data_selector.set_rpl(PrivilegeLevel::Ring3);
        DS::set_reg(data_selector.clone());
        data_selector.0
    };
    tlb::flush_all();
    asm!("\
    push rax   // stack segment
    push rsi   // rsp
    push 0x200 // rflags (only interrupt bit set)
    push rdx   // code segment
    push rdi   // ret to virtual addr
    iretq",
    in("rdi") code.as_u64(), in("rsi") stack_end.as_u64(), in("dx") cs_idx, in("ax") ds_idx);
}
