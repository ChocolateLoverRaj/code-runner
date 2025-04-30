use core::arch::naked_asm;

use x86_64::{
    registers::segmentation::GS,
    structures::{gdt::SegmentSelector, idt::InterruptStackFrame},
    PrivilegeLevel,
};

use crate::{
    context::{Context, FullContext},
    cpu_local_data::get_local,
    hpet::HPET,
    hpet_memory::{
        HpetGeneralInterruptStatusRegister, HpetMemoryVolatileFieldAccess,
        HpetTimerMemoryVolatileFieldAccess,
    },
};

/// # Safety
/// This function should only be called as a HPET interrupt handler, and not manually.
#[naked]
pub unsafe extern "sysv64" fn hpet_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        naked_asm!("\
            push r15
            push r14
            push r13
            push r12
            push r11
            push r10
            push r9
            push r8
            push rdi
            push rsi
            push rdx
            push rcx
            push rbx
            push rax
            push rbp

            mov rdi, rsp   // first arg of context switch is the context which is all the registers saved above

            // The function should never return
            call {context_switch}
            // asm! version of unreachable!()
            ud2
            ",
            context_switch = sym hpet_interrupt_handler_rust
        );
    };
}

unsafe extern "sysv64" fn hpet_interrupt_handler_rust(context: &FullContext) -> ! {
    // This is to make sure that GS.Base is set to the kernel's gs base
    let swapped_gs =
        if SegmentSelector(context.cs.try_into().unwrap()).rpl() == PrivilegeLevel::Ring3 {
            unsafe { GS::swap() };
            true
        } else {
            false
        };
    {
        let mut hpet = HPET.try_get().unwrap().write();
        let interrupt_status_register = hpet.as_ptr().interrupt_status().read();
        if interrupt_status_register.0 == 0 {
            log::debug!("Received timer interrupt when no timer actually fired. Assuming it's from PIT and ignoring.");
        }
        // log::info!(
        //     "Interrupt status register before clearing: {:?}",
        //     hpet.as_ptr().interrupt_status().read()
        // );

        for i in 0..=hpet.as_ptr().capabilities_and_id().read().get_num_tim_cap() {
            let received_interrupt = interrupt_status_register.get_t_n_int_sts(i as usize) == 1;
            if received_interrupt {
                log::info!("Received interrupt from timer: {}", i);
                // interrupt_status_register.set_t_n_int_sts(i as usize, 0);
                hpet.as_mut_ptr()
                    .interrupt_status()
                    .write(HpetGeneralInterruptStatusRegister(1 << i));
                hpet.as_mut_ptr()
                    .timers()
                    .as_slice()
                    .index(i as usize)
                    .configuration_and_capability_register()
                    .update(|mut r| {
                        // At this point the counter >= comparator
                        // Since the interrupt type is level, we will continue receiving interrupts and the interrupt status bit will continue to be set.
                        // We don't want that, so we can just set the trigger type to edge, and we won't get more interrupts unless we change the comparator value to >counter.
                        r.set_int_type_cnf(false);
                        r.set_int_enb_cnf(false);
                        r
                    });
            }
        }
        // log::info!(
        //     "Interrupt status register after clearing: {:?}",
        //     hpet.as_ptr().interrupt_status().read()
        // );
    }
    {
        let mut local_apic = get_local().unwrap().local_apic.try_get().unwrap().lock();
        // Safety: This interrupt was dispatched by the local APIC and we are notifying that it's done.
        unsafe { local_apic.end_of_interrupt() };
    }
    if swapped_gs {
        unsafe { GS::swap() };
    }
    unsafe { context.restore() }
}
