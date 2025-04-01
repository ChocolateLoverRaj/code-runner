use core::mem::MaybeUninit;

use alloc::boxed::Box;
use spinning_top::Spinlock;
use util::init_later::InitLater;
use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x86_64::{
    structures::{
        idt::{self},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use crate::{
    cpu_local::CpuLocal,
    hhdm_offset::HhdmOffset,
    map_local_xapic::{map_local_xapic, LocalXapicVirtAddr},
    modules::{
        double_fault_handler_entry::get_double_fault_entry, gdt::Gdt, idt::IdtBuilder,
        logging_breakpoint_handler::logging_breakpoint_handler,
        panicking_double_fault_handler::panicking_double_fault_handler,
        panicking_general_protection_fault_handler::panicking_general_protection_fault_handler,
        panicking_invalid_opcode_handler::panicking_invalid_opcode_handler,
        panicking_invalid_tss_fault_handler::panicking_invalid_tss_fault_handler,
        panicking_local_apic_error_interrupt_handler::panicking_local_apic_error_interrupt_handler,
        panicking_page_fault_handler::panicking_page_fault_handler,
        panicking_segment_not_present_handler::panicking_segment_not_present_handler,
        panicking_spurious_interrupt_handler::panicking_spurious_interrupt_handler,
        panicking_stack_segment_fault_handler::panicking_stack_segment_fault_handler,
        spurious_interrupt_handler::set_spurious_interrupt_handler, tss::TssBuilder,
    },
    nmi_handler::nmi_handler,
    rsdp_addr::RsdpAddr,
    store_but_borrow_mut::StoreButBorrowMut,
    tasks::StackChunk,
    IOPB_SIZE,
};
static LOCAL_APIC_ADDR: InitLater<Option<LocalXapicVirtAddr>> = InitLater::uninit();

#[derive(Debug)]
struct StaticStuff0 {
    double_fault_handler_stack: Box<[MaybeUninit<StackChunk>]>,
}

static CPU_LOCAL_STATIC_STUFF_0: CpuLocal<StoreButBorrowMut<StaticStuff0>> = CpuLocal::uninit();

#[derive(Debug)]
struct StaticStuff1 {
    tss: TaskStateSegment<IOPB_SIZE>,
    idt_builder: IdtBuilder,
    spurious_interrupt_handler_index: u8,
    timer_interrupt_index: u8,
    local_apic_error_interrupt_index: u8,
    /// This is to make sure that the privileged TSS stack is not dropped during the kernel's execution
    priv_tss_stack: Box<[MaybeUninit<u8>]>,
}

static CPU_LOCAL_STATIC_STUFF_1: CpuLocal<StoreButBorrowMut<StaticStuff1>> = CpuLocal::uninit();

#[derive(Debug)]
struct StaticStuff2 {
    gdt: Gdt,
    iopb: Spinlock<&'static mut [u8; IOPB_SIZE]>,
}

static CPU_LOCAL_STATIC_STUFF_2: CpuLocal<InitLater<StaticStuff2>> = CpuLocal::uninit();

pub static CPU_LOCAL_APICS: CpuLocal<InitLater<Spinlock<LocalApic>>> = CpuLocal::uninit();

pub fn init_vars_for_idt_and_gdt(rsdp_addr: RsdpAddr, hhdm_offset: HhdmOffset) {
    let acpi_tables = crate::acpi::init(rsdp_addr, hhdm_offset).unwrap();
    let local_apic_addr = map_local_xapic(&acpi_tables.lock(), hhdm_offset).unwrap();
    LOCAL_APIC_ADDR.try_init(local_apic_addr).unwrap();
    CPU_LOCAL_STATIC_STUFF_0
        .try_init(StoreButBorrowMut::uninit)
        .unwrap();
    CPU_LOCAL_STATIC_STUFF_1
        .try_init(StoreButBorrowMut::uninit)
        .unwrap();
    CPU_LOCAL_STATIC_STUFF_2
        .try_init(InitLater::uninit)
        .unwrap();
    CPU_LOCAL_APICS.try_init(InitLater::uninit).unwrap();
}

pub fn init_idt_and_gdt() {
    let static_stuff_0 = CPU_LOCAL_STATIC_STUFF_0
        .try_get()
        .unwrap()
        .store_but_borrow_mut(StaticStuff0 {
            double_fault_handler_stack: Box::new_uninit_slice(0x200),
        })
        .unwrap();
    let static_stuff_1 = CPU_LOCAL_STATIC_STUFF_1
        .try_get()
        .unwrap()
        .store_but_borrow_mut({
            let mut tss = TssBuilder::<IOPB_SIZE>::default();
            let mut idt_builder = IdtBuilder::default();
            idt_builder
                .set_double_fault_entry(get_double_fault_entry(
                    &mut tss,
                    panicking_double_fault_handler,
                    Gdt::cs(),
                    &mut static_stuff_0.double_fault_handler_stack,
                ))
                .unwrap();
            idt_builder
                .set_breakpoint_entry(idt::Entry::from_handler_fn(
                    logging_breakpoint_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            idt_builder
                .set_general_protection_fault_entry(idt::Entry::from_handler_fn(
                    panicking_general_protection_fault_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            idt_builder
                .set_page_fault_entry(idt::Entry::from_handler_fn(
                    panicking_page_fault_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            idt_builder
                .set_invalid_tss_fault_entry(idt::Entry::from_handler_fn(
                    panicking_invalid_tss_fault_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            idt_builder
                .set_security_exception_fault_entry(idt::Entry::from_handler_fn(
                    panicking_general_protection_fault_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            idt_builder
                .set_segment_not_present_entry(idt::Entry::from_handler_fn(
                    panicking_segment_not_present_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            idt_builder
                .set_invalid_opcode_entry(idt::Entry::from_handler_fn(
                    panicking_invalid_opcode_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            idt_builder
                .set_stack_segment_fault_entry(idt::Entry::from_handler_fn(
                    panicking_stack_segment_fault_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();
            let spurious_interrupt_handler_index = set_spurious_interrupt_handler(
                &mut idt_builder,
                panicking_spurious_interrupt_handler,
                idt::EntryOptions::present_with_cs(Gdt::cs()),
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
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();

            idt_builder
                .set_non_maskable_interrupt_entry(idt::Entry::from_handler_fn(
                    nmi_handler,
                    idt::EntryOptions::present_with_cs(Gdt::cs()),
                ))
                .unwrap();

            /// This is the stack that gets switched to when an interrupt handler is called while the CPU is in user mode
            const PRIV_TSS_STACK_SIZE: usize = 0x2000;
            let mut priv_tss_stack = Box::<[u8]>::new_uninit_slice(PRIV_TSS_STACK_SIZE);
            tss.add_privilege_stack_table_entry({
                let stack_start = VirtAddr::from_ptr(priv_tss_stack.as_mut_ptr());
                stack_start + PRIV_TSS_STACK_SIZE as u64
            })
            .unwrap();
            let tss = tss.get_tss();
            StaticStuff1 {
                tss,
                idt_builder,
                spurious_interrupt_handler_index,
                timer_interrupt_index,
                local_apic_error_interrupt_index,
                priv_tss_stack,
            }
        })
        .unwrap();
    let static_stuff_2 = CPU_LOCAL_STATIC_STUFF_2
        .try_get()
        .unwrap()
        .try_init({
            let (tss_pointer, iopb) = static_stuff_1.tss.ready_to_activate();
            let gdt = Gdt::new(tss_pointer);
            StaticStuff2 {
                gdt,
                iopb: Spinlock::new(iopb),
            }
        })
        .unwrap();
    static_stuff_2.gdt.init();
    static_stuff_1.idt_builder.init();
    CPU_LOCAL_APICS
        .try_get()
        .unwrap()
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

pub fn get_iobp() -> &'static Spinlock<&'static mut [u8; IOPB_SIZE]> {
    &CPU_LOCAL_STATIC_STUFF_2
        .try_get()
        .unwrap()
        .try_get()
        .unwrap()
        .iopb
}
