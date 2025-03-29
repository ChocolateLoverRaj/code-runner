use core::{arch::asm, mem::MaybeUninit, ptr::NonNull};

use acpi::AcpiHandler;
use alloc::boxed::Box;
use limine::response::MpResponse;
use spinning_top::Spinlock;
use util::init_later::InitLater;
use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x86_64::{
    instructions::interrupts,
    structures::{
        idt::{self, InterruptStackFrame},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

use crate::{
    acpi::ACPI_TABLES,
    hhdm_offset::HhdmOffset,
    hlt_loop::hlt_loop,
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
    nmi_handler::{self, nmi_handler},
    rsdp_addr::RsdpAddr,
    store_but_borrow_mut::StoreButBorrowMut,
    tasks::StackChunk,
    IOPB_SIZE,
};

static LOCAL_XAPIC: InitLater<Option<LocalXapicVirtAddr>> = InitLater::uninit();

#[derive(Debug)]
struct StaticStuff0 {
    double_fault_handler_stack: Box<[MaybeUninit<StackChunk>]>,
}

static CPU_LOCAL_STATIC_STUFF_0: InitLater<Box<[StoreButBorrowMut<StaticStuff0>]>> =
    InitLater::uninit();

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

static CPU_LOCAL_STATIC_STUFF_1: InitLater<Box<[StoreButBorrowMut<StaticStuff1>]>> =
    InitLater::uninit();

#[derive(Debug)]
struct StaticStuff2 {
    gdt: Gdt,
    iopb: Spinlock<&'static mut [u8; IOPB_SIZE]>,
}

static CPU_LOCAL_STATIC_STUFF_2: InitLater<Box<[StoreButBorrowMut<StaticStuff2>]>> =
    InitLater::uninit();

pub static CPU_LOCAL_APICS: InitLater<Box<[InitLater<Spinlock<LocalApic>>]>> = InitLater::uninit();

pub fn init_cpus(mp_response: &mut MpResponse, rsdp_addr: RsdpAddr, hhdm_offset: HhdmOffset) -> ! {
    let acpi_tables = crate::acpi::init(rsdp_addr, hhdm_offset).unwrap();
    let local_xapic = map_local_xapic(&acpi_tables.lock(), hhdm_offset).unwrap();
    LOCAL_XAPIC.try_init(local_xapic);

    let cpu_count = mp_response.cpus().len();

    CPU_LOCAL_STATIC_STUFF_0
        .try_init(
            (0..cpu_count)
                .map(|_| StoreButBorrowMut::uninit())
                .collect(),
        )
        .unwrap();
    CPU_LOCAL_STATIC_STUFF_1
        .try_init(
            (0..cpu_count)
                .map(|_| StoreButBorrowMut::uninit())
                .collect(),
        )
        .unwrap();
    CPU_LOCAL_STATIC_STUFF_2
        .try_init(
            (0..cpu_count)
                .map(|_| StoreButBorrowMut::uninit())
                .collect(),
        )
        .unwrap();
    CPU_LOCAL_APICS
        .try_init((0..cpu_count).map(|_| InitLater::uninit()).collect())
        .unwrap();
    mp_response.cpus_mut().iter_mut().for_each(|cpu| {
        cpu.goto_address.write(init_cpu);
    });

    let current_cpu = mp_response
        .cpus()
        .iter()
        .find(|cpu| cpu.lapic_id == mp_response.bsp_lapic_id())
        .unwrap();
    unsafe { init_cpu(current_cpu) }
}

unsafe extern "C" fn init_cpu(cpu: &limine::mp::Cpu) -> ! {
    // This is probably needed cuz the allocator changed page tables. It did not change the location of the L4 page table though.
    x86_64::instructions::tlb::flush_all();

    log::info!("Hello from CPU: {:?}. LAPIC ID: {:?}", cpu.id, cpu.lapic_id);

    // let _array = [0_u8; 0x10000];
    // let b = CPU_LOCAL_TEST.try_get().unwrap();
    // log::info!("Box: {:p}", b.deref());
    // x86_64::instructions::tlb::flush_all();
    // log::info!("Box: {:?}", b[cpu.id as usize].store_but_borrow_mut(3));

    let static_stuff_0 = CPU_LOCAL_STATIC_STUFF_0.try_get().unwrap()[cpu.id as usize]
        .store_but_borrow_mut(StaticStuff0 {
            double_fault_handler_stack: Box::new_uninit_slice(0x200),
        })
        .unwrap();
    let static_stuff_1 = CPU_LOCAL_STATIC_STUFF_1.try_get().unwrap()[cpu.id as usize]
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
    let static_stuff_2 = CPU_LOCAL_STATIC_STUFF_2.try_get().unwrap()[cpu.id as usize]
        .store_but_borrow_mut({
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

    log::info!("Initialized GDT and IDT on CPU {}", cpu.id);

    let mut lapic = CPU_LOCAL_APICS.try_get().unwrap()[cpu.id as usize]
        .try_init({
            let mut builder = LocalApicBuilder::new();
            builder
                .timer_vector(static_stuff_1.timer_interrupt_index as usize)
                .spurious_vector(static_stuff_1.spurious_interrupt_handler_index as usize)
                .error_vector(static_stuff_1.local_apic_error_interrupt_index as usize);
            if let Some(local_xapic) = LOCAL_XAPIC.try_get().unwrap() {
                builder.set_xapic_base(VirtAddr::from(*local_xapic).as_u64());
            }
            let mut local_apic = builder.build().unwrap();
            unsafe { local_apic.enable() };
            unsafe { local_apic.disable_timer() };
            Spinlock::new(local_apic)
        })
        .unwrap();

    log::info!("Initialized Local APIC on CPU {}", cpu.id);

    if cpu.id == 0 {
        panic!("Test panic");
    }

    loop {
        log::info!("Log from CPU {:?}", cpu.id);
    }

    hlt_loop()
}
