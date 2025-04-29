use spinning_top::Spinlock;
use util::init_later::InitLater;
use x2apic::{ioapic::IoApic, lapic::LocalApicBuilder};
use x86_64::{
    structures::{
        idt::{self},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use crate::{
    boxed_stack::BoxedStack,
    cpu_local_data::{self, get_local},
    interrupt_handlers::{
        breakpoint::breakpoint_handler, double_fault::double_fault_handler,
        gp_fault::gp_fault_handler, hpet::hpet_interrupt_handler,
        invalid_opcode_fault::invalid_opcode_handler, keyboard::keyboard_interrupt_handler,
        page_fault::page_fault_handler, segment_not_present::segment_not_present_handler,
    },
    io_permission_bitmap::IoPermissionBitmap,
    iopb_size::IOPB_SIZE,
    map_local_xapic::LocalXapicVirtAddr,
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

pub struct ApicData {
    pub io_apic: Spinlock<IoApic>,
    pub local_xapic: Option<LocalXapicVirtAddr>,
}

pub static MAPPED_APICS: InitLater<ApicData> = InitLater::uninit();

#[derive(Debug)]
pub struct StaticStuff1 {
    tss: TaskStateSegment<IOPB_SIZE>,
    idt_builder: IdtBuilder,
    spurious_interrupt_handler_index: u8,
    timer_interrupt_index: u8,
    local_apic_error_interrupt_index: u8,
}

#[derive(Debug)]
pub struct StaticStuff2 {
    gdt: Gdt,
    iopb: Spinlock<&'static mut [u8; IOPB_SIZE]>,
    /// The that the CPU uses for the double fault handler
    #[allow(unused)]
    double_fault_handler_stack: BoxedStack,
    /// The stack that the CPU uses for other fault handlers
    #[allow(unused)]
    other_fault_handler_stack: BoxedStack,
    /// The stack that the CPU uses when transitioning from user mode to kernel mode to call an interrupt handler
    priv_tss_stack: BoxedStack,
    pub keyboard_interrupt_index: u8,
    hpet_interrupt_index: u8,
}

pub fn init_cpu() {
    let idt_stack_size = 0x10_000;
    let priv_tss_stack = BoxedStack::new_uninit(idt_stack_size);
    let double_fault_handler_stack = BoxedStack::new_uninit(idt_stack_size);
    let other_fault_handler_stack = BoxedStack::new_uninit(idt_stack_size);
    let cpu_local_data = cpu_local_data::get_local().unwrap();

    let mut tss = TssBuilder::<IOPB_SIZE>::default();
    let mut idt_builder = IdtBuilder::default();
    let double_fault_stack_index = tss
        .add_interrupt_stack_table_entry(double_fault_handler_stack.top())
        .unwrap();
    let other_fault_stack_index = tss
        .add_interrupt_stack_table_entry(other_fault_handler_stack.top())
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
    let keyboard_interrupt_index = idt_builder
        .set_flexible_entry(idt::Entry::from_handler_addr(
            VirtAddr::from_ptr(keyboard_interrupt_handler as *const ()),
            idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
        ))
        .unwrap();
    let hpet_interrupt_index = idt_builder
        .set_flexible_entry(idt::Entry::from_handler_addr(
            VirtAddr::from_ptr(hpet_interrupt_handler as *const ()),
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
    tss.set_privilege_stack_table_entry_from_ring_3(priv_tss_stack.top())
        .unwrap();
    let tss = tss.get_tss();
    let static_stuff_1 = StaticStuff1 {
        tss,
        idt_builder,
        spurious_interrupt_handler_index,
        timer_interrupt_index,
        local_apic_error_interrupt_index,
    };
    let static_stuff_1 = cpu_local_data
        .static_stuff1
        .store_but_borrow_mut(static_stuff_1)
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
                keyboard_interrupt_index,
                hpet_interrupt_index,
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
            if let Some(local_xapic) = MAPPED_APICS.try_get().unwrap().local_xapic {
                builder.set_xapic_base(VirtAddr::from(local_xapic).as_u64());
            }
            let mut local_apic = builder.build().unwrap();
            unsafe { local_apic.enable() };
            unsafe { local_apic.disable_timer() };
            Spinlock::new(local_apic)
        })
        .unwrap();
}

pub fn get_iopb() -> &'static Spinlock<&'static mut IoPermissionBitmap<IOPB_SIZE>> {
    unsafe { core::mem::transmute(&get_local().unwrap().static_stuff2.try_get().unwrap().iopb) }
}

pub fn get_priv_stack() -> &'static BoxedStack {
    &get_local()
        .unwrap()
        .static_stuff2
        .try_get()
        .unwrap()
        .priv_tss_stack
}
