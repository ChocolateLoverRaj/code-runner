use core::{arch::naked_asm, ptr::NonNull};

use acpi::{AcpiHandler, AcpiTables, HpetInfo};
use anyhow::anyhow;
use conquer_once::noblock::OnceCell;
use spin::Mutex;
use volatile::{VolatilePtr, VolatileRef};
use x2apic::{
    ioapic::{IoApic, RedirectionTableEntry},
    lapic::LocalApic,
};
use x86_64::{
    structures::{
        idt::{Entry, HandlerFunc, InterruptStackFrame},
        paging::{frame::PhysFrameRange, PageSize, PageTableFlags, PhysFrame, Size4KiB},
    },
    PhysAddr, VirtAddr,
};

use crate::{
    context::{Context, FullContext},
    hpet_memory::{HpetMemory, HpetMemoryVolatileFieldAccess, HpetTimerMemoryVolatileFieldAccess},
    modules::{idt::IdtBuilder, unsafe_local_apic::UnsafeLocalApic},
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

static LOCAL_APIC: OnceCell<&'static Mutex<UnsafeLocalApic>> = OnceCell::uninit();

unsafe extern "sysv64" fn hpet_interrupt_handler(context: *const FullContext) -> ! {
    let context = unsafe { *context };
    log::info!("HPET Interrupt!");
    unsafe { LOCAL_APIC.try_get().unwrap().lock().end_of_interrupt() };
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

#[derive(Default)]
pub struct HpetBuilderStage0;

impl HpetBuilderStage0 {
    pub fn set_interrupt(self, idt_builder: &mut IdtBuilder) -> Option<HpetBuilderStage1> {
        let interrupt_index = idt_builder.set_flexible_entry({
            let mut entry = Entry::<HandlerFunc>::missing();
            unsafe {
                entry.set_handler_addr(VirtAddr::from_ptr(raw_hpet_interrupt_handler as *const ()));
            };
            entry
        })?;
        log::info!("HPET interrupt index: 0x{:x}", interrupt_index);
        Some(HpetBuilderStage1 { interrupt_index })
    }
}

pub struct HpetBuilderStage1 {
    interrupt_index: u8,
}

impl HpetBuilderStage1 {
    pub fn configure_io_apic(
        &'static self,
        hpet: VolatilePtr<HpetMemory>,
        io_apic: &mut IoApic,
        local_apic: &'static Mutex<UnsafeLocalApic>,
    ) -> Option<()> {
        let timer0 = hpet.timers().as_slice().index(0);

        let r = timer0.configuration_and_capability_register().read();
        let first_route = {
            let routes = r.get_int_route_cap();
            let mut i = 0_u8;
            loop {
                if i == 32 {
                    break None;
                }
                if routes & (1 << i) != 0 {
                    break Some(i);
                }
                i += 1;
            }
        }
        .unwrap();
        let first_route = 20;
        let m = unsafe { io_apic.max_table_entry() };

        log::info!("First route IRQ: {}", first_route);

        log::info!("Max table entry: {}", m);
        timer0.configuration_and_capability_register().write({
            let mut r = r;
            r.set_int_route_cnf(first_route);
            r.set_int_enb_cnf(true);
            r
        });

        let existing_table_entry = unsafe { io_apic.table_entry(first_route) };
        log::info!("Existing table entry: {:#?}", existing_table_entry);

        unsafe {
            io_apic.set_table_entry(first_route, {
                let mut entry = RedirectionTableEntry::default();
                entry.set_vector(self.interrupt_index);
                entry
            });
            io_apic.enable_irq(first_route);
        };

        LOCAL_APIC.try_init_once(|| local_apic).unwrap();

        timer0.comparator_register().write(500_000_000);

        Some(())
    }
}
