use spinning_top::Spinlock;
use x2apic::lapic::LocalApicBuilder;
use x86_64::{
    structures::{
        idt::{self, HandlerFunc, InterruptDescriptorTable},
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
    interrupt_numbers::InterruptNumbers,
    io_permission_bitmap::IoPermissionBitmap,
    iopb_size::IOPB_SIZE,
    mapped_apics::MAPPED_APICS,
    modules::{
        gdt::Gdt, panicking_invalid_tss_fault_handler::panicking_invalid_tss_fault_handler,
        panicking_local_apic_error_interrupt_handler::panicking_local_apic_error_interrupt_handler,
        panicking_spurious_interrupt_handler::panicking_spurious_interrupt_handler,
        panicking_stack_segment_fault_handler::panicking_stack_segment_fault_handler,
        tss::TssBuilder,
    },
    nmi_handler::nmi_handler,
};

#[derive(Debug)]
pub struct StaticStuff1 {
    tss: TaskStateSegment<IOPB_SIZE>,
    idt: InterruptDescriptorTable,
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
}

pub fn init_cpu() {
    let idt_stack_size = 0x10_000;
    let priv_tss_stack = BoxedStack::new_uninit(idt_stack_size);
    let double_fault_handler_stack = BoxedStack::new_uninit(idt_stack_size);
    let other_fault_handler_stack = BoxedStack::new_uninit(idt_stack_size);
    let cpu_local_data = cpu_local_data::get_local().unwrap();

    let mut tss = TssBuilder::<IOPB_SIZE>::default();
    let mut idt = InterruptDescriptorTable::default();
    let double_fault_stack_index = tss
        .add_interrupt_stack_table_entry(double_fault_handler_stack.top())
        .unwrap();
    let other_fault_stack_index = tss
        .add_interrupt_stack_table_entry(other_fault_handler_stack.top())
        .unwrap();
    idt.double_fault = idt::Entry::from_handler_fn(
        double_fault_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), double_fault_stack_index),
    );
    idt.breakpoint = idt::Entry::from_handler_fn(
        breakpoint_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.general_protection_fault = idt::Entry::from_handler_fn(
        gp_fault_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.page_fault = idt::Entry::from_handler_fn(
        page_fault_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.invalid_tss = idt::Entry::from_handler_fn(
        panicking_invalid_tss_fault_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.security_exception = idt::Entry::from_handler_fn(
        gp_fault_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.segment_not_present = idt::Entry::from_handler_fn(
        segment_not_present_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.invalid_opcode = idt::Entry::from_handler_fn(
        invalid_opcode_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.stack_segment_fault = idt::Entry::from_handler_fn(
        panicking_stack_segment_fault_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt.non_maskable_interrupt = idt::Entry::from_handler_fn(
        nmi_handler,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );

    idt[u8::from(InterruptNumbers::LocalApicSpurious)] = idt::Entry::from_handler_fn(
        panicking_spurious_interrupt_handler as HandlerFunc,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    // Since we aren't using the Local APIC Timer and we are disabling it, it will never fire the interrupt and we don't need a handler for it.
    idt[u8::from(InterruptNumbers::LocalApicTimer)] = idt::Entry::missing();
    idt[u8::from(InterruptNumbers::LocalApicError)] = idt::Entry::from_handler_fn(
        panicking_local_apic_error_interrupt_handler as HandlerFunc,
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt[u8::from(InterruptNumbers::Keyboard)] = idt::Entry::from_handler_addr(
        VirtAddr::from_ptr(keyboard_interrupt_handler as *const ()),
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );
    idt[u8::from(InterruptNumbers::Hpet)] = idt::Entry::from_handler_addr(
        VirtAddr::from_ptr(hpet_interrupt_handler as *const ()),
        idt::EntryOptions::present_with_cs_and_ist(Gdt::cs(), other_fault_stack_index),
    );

    // This is the stack that gets switched to when an interrupt handler is called while the CPU is in user mode
    tss.set_privilege_stack_table_entry_from_ring_3(priv_tss_stack.top())
        .unwrap();
    let tss = tss.get_tss();
    let static_stuff_1 = StaticStuff1 { tss, idt };
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
            }
        })
        .unwrap();
    static_stuff_2.gdt.init();
    static_stuff_1.idt.load();
    cpu_local_data
        .local_apic
        .try_init({
            let mut builder = LocalApicBuilder::new();
            builder
                .timer_vector(u8::from(InterruptNumbers::LocalApicTimer).into())
                .spurious_vector(u8::from(InterruptNumbers::LocalApicSpurious).into())
                .error_vector(u8::from(InterruptNumbers::LocalApicError).into());
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
