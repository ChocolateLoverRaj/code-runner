use core::{arch::naked_asm, ptr::NonNull};

use acpi::{AcpiHandler, AcpiTables, HpetInfo};
use anyhow::anyhow;
use volatile::VolatileRef;
use x86_64::{
    structures::{
        idt::InterruptStackFrame,
        paging::{frame::PhysFrameRange, PageSize, PageTableFlags, PhysFrame, Size4KiB},
    },
    PhysAddr,
};

use crate::{
    context::{Context, FullContext},
    hpet_memory::HpetMemory,
    phys_mapper::PhysMapper,
};

#[naked]
unsafe extern "sysv64" fn raw_hpet_interrupt_handler(_stack_frame: InterruptStackFrame) {
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
            call {rust}
            // asm! version of unreachable!() 
            ud2
            ", 
            rust = sym hpet_interrupt_handler
        );
    };
}

unsafe extern "sysv64" fn hpet_interrupt_handler(context: *const FullContext) -> ! {
    let context = unsafe { *context };
    unsafe { context.restore() }
}

pub fn init<H: AcpiHandler>(
    acpi_tables: &AcpiTables<H>,
    phys_mapper: PhysMapper,
) -> anyhow::Result<VolatileRef<'static, HpetMemory>> {
    let hpet_info = HpetInfo::new(acpi_tables).map_err(|e| anyhow!("{:?}", e))?;
    let virt_mapping = unsafe {
        phys_mapper.map_to_phys(
            {
                let start = PhysAddr::new(hpet_info.base_address as u64);
                PhysFrameRange {
                    start: PhysFrame::containing_address(start),
                    end: PhysFrame::containing_address(start + size_of::<HpetMemory>() as u64) + 1,
                }
            },
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE
                | PageTableFlags::NO_EXECUTE,
        )
    };
    log::info!("HPET INfo: {:#?}", hpet_info);
    let virt_start =
        virt_mapping.start.start_address() + (hpet_info.base_address as u64 % Size4KiB::SIZE);

    // Safety: The pointer is pointing to the start address of the HPET and we will never unmap the pages
    let hpet_volatile_ref =
        unsafe { VolatileRef::<HpetMemory>::new(NonNull::new(virt_start.as_mut_ptr()).unwrap()) };

    Ok(hpet_volatile_ref)
}
