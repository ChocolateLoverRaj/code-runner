use acpi::platform::interrupt::Apic;
use alloc::alloc::Global;
use anyhow::anyhow;
use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x86_64::{
    structures::paging::{PageTableFlags, PhysFrame},
    PhysAddr,
};

use crate::phys_mapper::PhysMapper;

pub fn get_local_apic(
    apic: &Apic<Global>,
    phys_mapper: &mut PhysMapper,
    spurious_interrupt_index: u8,
    timer_interrupt_index: u8,
    error_interrupt_index: u8,
) -> anyhow::Result<LocalApic> {
    let local_mapping = unsafe {
        phys_mapper.map_to_phys(
            {
                let frame =
                    PhysFrame::containing_address(PhysAddr::new(apic.local_apic_address as u64));
                PhysFrame::range(frame, frame + 1)
            },
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE
                | PageTableFlags::NO_EXECUTE,
        )
    };
    let local_apic = LocalApicBuilder::new()
        .spurious_vector(spurious_interrupt_index as usize)
        .timer_vector(timer_interrupt_index as usize)
        // .timer_mode(TimerMode::Periodic)
        // .timer_divide(TimerDivide::Div16)
        // .timer_initial(0x5000000) // This can be anything, I chose this so that it interrupts every ~1.5 seconds
        .error_vector(error_interrupt_index as usize)
        .set_xapic_base(local_mapping.start.start_address().as_u64())
        .build()
        .map_err(|e| anyhow!("{e}"))?;
    Ok(local_apic)
}
