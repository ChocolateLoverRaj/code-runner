use core::ops::Range;

use acpi::{AcpiTables, InterruptModel};
use alloc::sync::Arc;
use anyhow::{anyhow, Context};
use conquer_once::spin::OnceCell;
use x2apic::{
    ioapic::{IoApic, RedirectionTableEntry},
    lapic::{LocalApic, LocalApicBuilder, TimerDivide, TimerMode},
};
use x86_64::{
    instructions::interrupts::without_interrupts,
    structures::paging::{OffsetPageTable, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use crate::{interrupts::InterruptIndex, memory::BootInfoFrameAllocator, phys_mapper::PhysMapper};

pub static LOCAL_APIC: OnceCell<spin::Mutex<LocalApic>> = OnceCell::uninit();

pub unsafe fn init_apic(
    rsdp_addr: usize,
    mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
    virt_mem_allocator: Arc<spin::Mutex<alloc::vec::Vec<Range<VirtAddr>>>>,
    frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
) -> anyhow::Result<()> {
    log::info!("RSDP: {:?}", rsdp_addr);
    let phys_mapper = PhysMapper::new(mapper, virt_mem_allocator, frame_allocator);
    let acpi_tables = unsafe { AcpiTables::from_rsdp(phys_mapper.clone(), rsdp_addr) }
        .map_err(|e| anyhow!("{e:?}"))
        .context("Error reading ACPI tables")?;
    let platform_info = acpi_tables.platform_info().map_err(|e| anyhow!("{e:?}"))?;
    let interrupt_model = platform_info.interrupt_model;
    log::debug!("ACPI Tables: {:#?}", interrupt_model);
    match interrupt_model {
        InterruptModel::Apic(apic) => {
            without_interrupts(|| -> anyhow::Result<()> {
                log::debug!("Interrupt model: {apic:#?}");
                // Map IO APIC
                // From https://wiki.osdev.org/APIC#IO_APIC_Registers, there are 64 32-bit registers, so 256 bytes need to be mapped to access the IO APIC. We can map a single frame.
                let io_mapping = phys_mapper.map_to_phys(
                    {
                        let frame = PhysFrame::containing_address(PhysAddr::new(
                            apic.io_apics[0].address as u64,
                        ));
                        PhysFrame::range(frame, frame + 1)
                    },
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::NO_CACHE
                        | PageTableFlags::NO_EXECUTE,
                );
                let mut io_apic = IoApic::new(io_mapping.start.start_address().as_u64());
                // Enable keyboard interrupts
                io_apic.set_table_entry(1, {
                    let mut entry = RedirectionTableEntry::default();
                    entry.set_vector(InterruptIndex::Keyboard.into());
                    entry
                });
                io_apic.enable_irq(1);
                phys_mapper.unmap(io_mapping);

                let local_mapping = phys_mapper.map_to_phys(
                    {
                        let frame = PhysFrame::containing_address(PhysAddr::new(
                            apic.local_apic_address as u64,
                        ));
                        PhysFrame::range(frame, frame + 1)
                    },
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::NO_CACHE
                        | PageTableFlags::NO_EXECUTE,
                );
                let mut local_apic = LocalApicBuilder::new()
                    .spurious_vector(u8::from(InterruptIndex::Suprious) as usize)
                    .timer_vector(u8::from(InterruptIndex::Timer) as usize)
                    .timer_mode(TimerMode::Periodic)
                    .timer_divide(TimerDivide::Div16)
                    .timer_initial(0x5000000) // This can be anything, I chose this so that it interrupts every ~1.5 seconds
                    .error_vector(u8::from(InterruptIndex::LocalApicError) as usize)
                    .set_xapic_base(local_mapping.start.start_address().as_u64())
                    .build()
                    .map_err(|e| anyhow!("{e}"))?;
                local_apic.enable();
                LOCAL_APIC.init_once(|| spin::Mutex::new(local_apic));
                Ok(())
            })?;
        }
        _ => {
            log::warn!("Unknown interrupt model.");
        }
    }
    Ok(())
}
