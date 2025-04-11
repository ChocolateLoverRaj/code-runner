use core::{cell::RefCell, mem::MaybeUninit};

use acpi::{AcpiHandler, AcpiTables};
use alloc::boxed::Box;
use spinning_top::Spinlock;
use util::init_later::InitLater;
use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x86_64::{
    structures::{
        idt::{self},
        paging::{FrameAllocator, Size4KiB},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use crate::{
    boxed_stack::{BoxedStack, BoxedStackExt},
    cpu_local_data,
    fault_handlers::{
        breakpoint::breakpoint_handler, double_fault::double_fault_handler,
        gp_fault::gp_fault_handler, invalid_opcode_fault::invalid_opcode_handler,
        page_fault::page_fault_handler, segment_not_present::segment_not_present_handler,
    },
    hhdm_offset::HhdmOffset,
    iopb_size::IOPB_SIZE,
    map_local_xapic::{map_local_xapic, LocalXapicVirtAddr},
    modules::{
        gdt::Gdt, idt::IdtBuilder,
        panicking_invalid_tss_fault_handler::panicking_invalid_tss_fault_handler,
        panicking_local_apic_error_interrupt_handler::panicking_local_apic_error_interrupt_handler,
        panicking_spurious_interrupt_handler::panicking_spurious_interrupt_handler,
        panicking_stack_segment_fault_handler::panicking_stack_segment_fault_handler,
        spurious_interrupt_handler::set_spurious_interrupt_handler, tss::TssBuilder,
    },
    nmi_handler::nmi_handler,
};
static LOCAL_APIC_ADDR: InitLater<Option<LocalXapicVirtAddr>> = InitLater::uninit();

#[derive(Debug)]
pub struct StaticStuff1 {
    tss: TaskStateSegment<IOPB_SIZE>,
    idt_builder: IdtBuilder,
    spurious_interrupt_handler_index: u8,
    timer_interrupt_index: u8,
    local_apic_error_interrupt_index: u8,
}

// static CPU_LOCAL_STATIC_STUFF_1: CpuLocal<StoreButBorrowMut<StaticStuff1>> = CpuLocal::uninit();

#[derive(Debug)]
pub struct StaticStuff2 {
    gdt: Gdt,
    iopb: Spinlock<&'static mut [u8; IOPB_SIZE]>,
    /// The that the CPU uses for the double fault handler
    double_fault_handler_stack: BoxedStack,
    /// The stack that the CPU uses for other fault handlers
    other_fault_handler_stack: BoxedStack,
    /// The stack that the CPU uses when transitioning from user mode to kernel mode to call an interrupt handler
    priv_tss_stack: BoxedStack,
}

// static CPU_LOCAL_STATIC_STUFF_2: CpuLocal<InitLater<StaticStuff2>> = CpuLocal::uninit();

// pub static CPU_LOCAL_APICS: CpuLocal<InitLater<Spinlock<LocalApic>>> = CpuLocal::uninit();

pub fn init_bsp(
    acpi_tables: &AcpiTables<impl AcpiHandler>,
    hhdm_offset: HhdmOffset,
    frame_allocator: &RefCell<impl FrameAllocator<Size4KiB>>,
) {
    let local_apic_addr = map_local_xapic(acpi_tables, hhdm_offset, frame_allocator).unwrap();
    LOCAL_APIC_ADDR.try_init(local_apic_addr).unwrap();
}

pub fn init_cpu() {
    let idt_stack_size = 0x10_000;
    let priv_tss_stack = BoxedStack::new_uninit_stack(idt_stack_size);
    let double_fault_handler_stack = BoxedStack::new_uninit_stack(idt_stack_size);
    let other_fault_handler_stack = BoxedStack::new_uninit_stack(idt_stack_size);
    let cpu_local_data = cpu_local_data::get_local().unwrap();

    let static_stuff_1 = cpu_local_data
        .static_stuff1
        .store_but_borrow_mut({
            let mut tss = TssBuilder::<IOPB_SIZE>::default();
            let mut idt_builder = IdtBuilder::default();
            let double_fault_stack_index = tss
                .add_interrupt_stack_table_entry(VirtAddr::from_ptr(
                    double_fault_handler_stack.as_ptr_range().end,
                ))
                .unwrap();
            let other_fault_stack_index = tss
                .add_interrupt_stack_table_entry(VirtAddr::from_ptr(
                    other_fault_handler_stack.as_ptr_range().end,
                ))
                .unwrap();
            idt_builder
                .set_double_fault_entry(idt::Entry::from_handler_fn(
                    double_fault_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), double_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_breakpoint_entry(idt::Entry::from_handler_fn(
                    breakpoint_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_general_protection_fault_entry(idt::Entry::from_handler_fn(
                    gp_fault_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_page_fault_entry(idt::Entry::from_handler_fn(
                    page_fault_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_invalid_tss_fault_entry(idt::Entry::from_handler_fn(
                    panicking_invalid_tss_fault_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_security_exception_fault_entry(idt::Entry::from_handler_fn(
                    gp_fault_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_segment_not_present_entry(idt::Entry::from_handler_fn(
                    segment_not_present_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_invalid_opcode_entry(idt::Entry::from_handler_fn(
                    invalid_opcode_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            idt_builder
                .set_stack_segment_fault_entry(idt::Entry::from_handler_fn(
                    panicking_stack_segment_fault_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();
            let spurious_interrupt_handler_index = set_spurious_interrupt_handler(
                &mut idt_builder,
                panicking_spurious_interrupt_handler,
                idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
            )
            .unwrap();
            let timer_interrupt_index = idt_builder
                .set_flexible_entry({
                    // TODO: Maybe actually use the LAPIC timer interrupt?
                    idt::Entry::missing()
                })
                .unwrap();
            let local_apic_error_interrupt_index = idt_builder
                .set_flexible_entry(idt::Entry::from_handler_fn(
                    panicking_local_apic_error_interrupt_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();

            idt_builder
                .set_non_maskable_interrupt_entry(idt::Entry::from_handler_fn(
                    nmi_handler,
                    idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
                ))
                .unwrap();

            // This is the stack that gets switched to when an interrupt handler is called while the CPU is in user mode
            tss.set_privilege_stack_table_entry_from_ring_3(VirtAddr::from_ptr(
                priv_tss_stack.as_ptr_range().end,
            ))
            .unwrap();
            let tss = tss.get_tss();
            StaticStuff1 {
                tss,
                idt_builder,
                spurious_interrupt_handler_index,
                timer_interrupt_index,
                local_apic_error_interrupt_index,
            }
        })
        .unwrap();
    let static_stuff_2 = cpu_local_data
        .static_stuff2
        .try_init({
            let (tss_pointer, iopb) = static_stuff_1.tss.ready_to_activate();
            let gdt = Gdt::new(tss_pointer);
            StaticStuff2 {
                gdt,
                iopb: Spinlock::new(iopb),
                double_fault_handler_stack,
                other_fault_handler_stack,
                priv_tss_stack,
            }
        })
        .unwrap();
    static_stuff_2.gdt.init();
    static_stuff_1.idt_builder.init();
    cpu_local_data
        .local_apic
        .try_init({
            let mut builder = LocalApicBuilder::new();
            builder
                .timer_vector(static_stuff_1.timer_interrupt_index as usize)
                .spurious_vector(static_stuff_1.spurious_interrupt_handler_index as usize)
                .error_vector(static_stuff_1.local_apic_error_interrupt_index as usize);
            if let Some(local_xapic) = LOCAL_APIC_ADDR.try_get().unwrap() {
                builder.set_xapic_base(VirtAddr::from(*local_xapic).as_u64());
            }
            let mut local_apic = builder.build().unwrap();
            unsafe { local_apic.enable() };
            unsafe { local_apic.disable_timer() };
            Spinlock::new(local_apic)
        })
        .unwrap();
}

// pub fn get_iobp() -> &'static Spinlock<&'static mut [u8; IOPB_SIZE]> {
//     &CPU_LOCAL_STATIC_STUFF_2
//         .try_get()
//         .unwrap()
//         .try_get()
//         .unwrap()
//         .iopb
// }

// pub fn get_priv_stack() -> &'static Box<[MaybeUninit<StackChunk>]> {
//     &CPU_LOCAL_STATIC_STUFF_2
//         .try_get()
//         .unwrap()
//         .try_get()
//         .unwrap()
//         .priv_tss_stack
// }
