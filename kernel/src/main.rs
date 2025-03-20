#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(int_roundings)]
#![feature(naked_functions)]
#![feature(pointer_is_aligned_to)]
#![feature(unsigned_is_multiple_of)]
#![feature(vec_push_within_capacity)]
#![feature(never_type)]
#![feature(fn_traits)]
#![feature(maybe_uninit_uninit_array)]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod acpi;
pub mod allocator;
pub mod apic;
pub mod colorful_logger;
pub mod combined_logger;
pub mod context;
pub mod cool_keyboard_interrupt_handler;
pub mod demo_async;
pub mod demo_async_keyboard_drop;
pub mod demo_async_rtc_drop;
pub mod demo_maze_roller_game;
pub mod draw_rust;
pub mod dynamic_combined_logger;
pub mod embedded_graphics_writer;
pub mod enter_user_mode;
pub mod execute_future;
pub mod find_used_virt_addrs;
pub mod frame_buffer;
pub mod get_rgb_color;
pub mod hlt_loop;
pub mod hpet;
pub mod hpet_memory;
pub mod insert;
pub mod iopb_size;
pub mod logger;
pub mod logger_without_interrupts;
pub mod memory;
pub mod modules;
pub mod phys_mapper;
pub mod pic8259_interrupts;
pub mod run_tasks;
pub mod set_color;
pub mod spawn_task;
pub mod spcr;
pub mod split_draw_target;
pub mod syscall_enable_hpet;
pub mod syscall_get_hpet_main_counter_period;
// pub mod syscall_handler;
pub mod limine;
pub mod log_boot_time;
pub mod store_but_borrow_mut;
pub mod syscall_handler_closure;
pub mod syscall_handler_make_me_logger;
pub mod syscall_hpet_read_main_counter_value;
pub mod syscall_print_handler;
pub mod tasks;
pub mod user_space_state;
pub mod virt_addr_from_indexes;
pub mod virt_mem_tracker;
pub mod write_logger;
pub mod write_with_cr;

use ::limine::{framebuffer::MemoryModel, memory_map::EntryType};
use alloc::{boxed::Box, sync::Arc};
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use common::{mem::KERNEL_VIRT_MEM_START, ram_disk::RamDisk};
use conquer_once::noblock::OnceCell;
use cool_keyboard_interrupt_handler::CoolKeyboardBuilder;
use core::{mem::transmute, ops::DerefMut, panic::PanicInfo, slice};
#[allow(unused)]
use demo_async::demo_async;
#[allow(unused)]
use demo_async_keyboard_drop::demo_async_keyboard_drop;
#[allow(unused)]
use demo_async_rtc_drop::demo_async_rtc_drop;
#[allow(unused)]
use demo_maze_roller_game::demo_maze_roller_game;
#[allow(unused)]
use draw_rust::draw_rust;
use hlt_loop::hlt_loop;
use hpet::{HpetBuilderStage0, HpetBuilderStage1};
use hpet_memory::HpetMemory;
use iopb_size::IOPB_SIZE;
use limine::{
    BASE_REVISION, FRAME_BUFFER_REQUEST, HHDM_REQUEST, LIMINE_BOOTLOADER_INFO_REQUEST,
    MEMORY_MAP_REQUEST, MODULE_REQUEST, MP_REQUEST, RSDP_REQUEST,
};
use log_boot_time::log_boot_time;
#[allow(unused)]
use logger::init_logger_with_framebuffer;
use modules::{
    double_fault_handler_entry::get_double_fault_entry,
    gdt::Gdt,
    get_apic::get_apic,
    get_io_apic::get_io_apic,
    get_local_apic::get_local_apic,
    idt::IdtBuilder,
    logging_breakpoint_handler::logging_breakpoint_handler,
    logging_timer_interrupt_handler::get_logging_timer_interrupt_handler,
    panicking_double_fault_handler::panicking_double_fault_handler,
    panicking_general_protection_fault_handler::panicking_general_protection_fault_handler,
    panicking_invalid_opcode_handler::panicking_invalid_opcode_handler,
    panicking_invalid_tss_fault_handler::panicking_invalid_tss_fault_handler,
    panicking_local_apic_error_interrupt_handler::panicking_local_apic_error_interrupt_handler,
    panicking_page_fault_handler::panicking_page_fault_handler,
    panicking_segment_not_present_handler::panicking_segment_not_present_handler,
    panicking_spurious_interrupt_handler::panicking_spurious_interrupt_handler,
    panicking_stack_segment_fault_handler::panicking_stack_segment_fault_handler,
    spurious_interrupt_handler::set_spurious_interrupt_handler,
    static_local_apic::{self, LOCAL_APIC},
    syscall::{init_syscalls::init_syscalls, syscall_handler_closure::set_syscall_handler_closure},
    tss::TssBuilder,
    unsafe_local_apic::UnsafeLocalApic,
};
use phys_mapper::PhysMapper;
use run_tasks::run_tasks;
use spawn_task::spawn_task;
use spcr::replace_serial_logger_if_redirected;
use spin::{Mutex, RwLock};
use spinning_top::Spinlock;
use store_but_borrow_mut::StoreButBorrowMut;
use syscall_handler_closure::syscall_handler_closure;
use tasks::TASKS;
use volatile::VolatileRef;
use x86_64::{
    instructions::interrupts,
    structures::{
        idt::{self, HandlerFunc, HandlerFuncWithErrCode, PageFaultHandlerFunc},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // If we don't disable interrupts, code could run while we are in an invalid state. We are in an invalid state from now until reboot because of the panic.
    interrupts::disable();
    log::error!("{}", info);
    hlt_loop()
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = 1_000_000;
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    // Use higher half for kernel to have space in the lower parts for ELFs
    config.mappings.dynamic_range_start = Some(KERNEL_VIRT_MEM_START);
    config
};

entry_point!(kernel_main_old, config = &BOOTLOADER_CONFIG);

#[derive(Debug)]
struct StaticStuff0 {
    tss: TaskStateSegment<IOPB_SIZE>,
    idt_builder: IdtBuilder,
    spurious_interrupt_handler_index: u8,
    timer_interrupt_index: u8,
    local_apic_error_interrupt_index: u8,
    keyboard: CoolKeyboardBuilder,
    hpet: HpetBuilderStage1,
}

static STATIC_STUFF_0: StoreButBorrowMut<StaticStuff0> = StoreButBorrowMut::uninit();

struct StaticStuff1 {
    gdt: Gdt,
    iopb: Spinlock<&'static mut [u8; IOPB_SIZE]>,
}

static STATIC_STUFF_1: OnceCell<StaticStuff1> = OnceCell::uninit();

static HPET: OnceCell<RwLock<VolatileRef<HpetMemory>>> = OnceCell::uninit();

#[export_name = "kernel_main"]
unsafe extern "C" fn kernel_main() -> ! {
    assert!(BASE_REVISION.is_supported());

    init_logger_with_framebuffer(None);

    log::error!("This is what an error message looks like");
    log::warn!("This is what a warning message looks like");
    log::info!("This is what an info message looks like");
    log::debug!("This is what a debug message looks like");
    log::trace!("This is what a trace message looks like");

    let bootloader_info = LIMINE_BOOTLOADER_INFO_REQUEST
        .get_response()
        .expect("No Limine bootloader info");
    log::info!(
        "This kernel was loaded by bootloader {:?} version {:?}",
        bootloader_info.name(),
        bootloader_info.version()
    );

    let memory_map_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    memory_map_response.entries().iter().for_each(|entry| {
        log::info!(
            "Memory ({:?}) at 0x{:013X?}..0x{:013X}",
            unsafe { transmute::<_, u64>(entry.entry_type) },
            entry.base,
            entry.base + entry.length
        );
    });
    let total_usable_memory = memory_map_response
        .entries()
        .iter()
        .filter(|entry| entry.entry_type == EntryType::USABLE)
        .map(|entry| entry.length)
        .sum::<u64>();
    log::info!("Total usable memory: 0x{:X} bytes", total_usable_memory);

    let rsdp = RSDP_REQUEST.get_response().unwrap().address();
    log::info!("RSDP Address: 0x{:X}", rsdp);

    let frame_buffer_response = FRAME_BUFFER_REQUEST.get_response().unwrap();
    frame_buffer_response
        .framebuffers()
        .for_each(|frame_buffer| {
            log::info!(
                "Frame buffer at {:?} with size {}x{}. Is RGB? {}",
                frame_buffer.addr(),
                frame_buffer.width(),
                frame_buffer.height(),
                frame_buffer.memory_model() == MemoryModel::RGB
            )
        });

    let mp_response = MP_REQUEST.get_response().unwrap();
    log::info!("{} CPUs", mp_response.cpus().len());
    mp_response
        .cpus()
        .iter()
        .for_each(|cpu| log::info!("CPU with id: {} and LAPIC id: {}", cpu.id, cpu.lapic_id));

    log_boot_time();

    let ram_disk = MODULE_REQUEST
        .get_response()
        .and_then(|response| response.modules().first());
    match ram_disk {
        Some(ram_disk) => {
            log::info!(
                "Got ram disk at {:?} with len {:?}",
                ram_disk.addr(),
                ram_disk.size()
            );
        }
        None => {}
    }

    let hhdm_offset = HHDM_REQUEST
        .get_response()
        .expect("No HHDM response")
        .offset();
    log::info!("HHDM offset: 0x{:X}", hhdm_offset);

    hlt_loop()
}

fn kernel_main_old(boot_info: &'static mut BootInfo) -> ! {
    let mut frame_buffer = boot_info.framebuffer.as_mut();
    if let Some(frame_buffer) = frame_buffer.as_mut() {
        frame_buffer.buffer_mut().fill(0);
    }
    init_logger_with_framebuffer(None);

    log::info!(
        "Ramdisk len: {:?}. Ramdisk addr: {:?}",
        boot_info.ramdisk_len,
        boot_info.ramdisk_addr
    );
    let static_stuff = STATIC_STUFF_0
        .store_but_borrow_mut({
            let mut tss = TssBuilder::default();

            let mut idt_builder = IdtBuilder::default();
            idt_builder
                .set_double_fault_entry(get_double_fault_entry(
                    &mut tss,
                    panicking_double_fault_handler,
                ))
                .unwrap();
            idt_builder
                .set_breakpoint_entry({
                    let mut entry = idt::Entry::<HandlerFunc>::missing();
                    entry.set_handler_fn(logging_breakpoint_handler);
                    entry
                })
                .unwrap();
            idt_builder
                .set_general_protection_fault_entry({
                    let mut entry = idt::Entry::<HandlerFuncWithErrCode>::missing();
                    entry.set_handler_fn(panicking_general_protection_fault_handler);
                    entry
                })
                .unwrap();
            idt_builder
                .set_page_fault_entry({
                    let mut entry = idt::Entry::<PageFaultHandlerFunc>::missing();
                    entry.set_handler_fn(panicking_page_fault_handler);
                    entry
                })
                .unwrap();
            idt_builder
                .set_invalid_tss_fault_entry({
                    let mut entry = idt::Entry::<HandlerFuncWithErrCode>::missing();
                    entry.set_handler_fn(panicking_invalid_tss_fault_handler);
                    entry
                })
                .unwrap();
            idt_builder
                .set_security_exception_fault_entry({
                    let mut entry = idt::Entry::<HandlerFuncWithErrCode>::missing();
                    entry.set_handler_fn(panicking_general_protection_fault_handler);
                    entry
                })
                .unwrap();
            idt_builder
                .set_segment_not_present_entry({
                    let mut entry = idt::Entry::<HandlerFuncWithErrCode>::missing();
                    entry.set_handler_fn(panicking_segment_not_present_handler);
                    entry
                })
                .unwrap();
            idt_builder
                .set_invalid_opcode_entry({
                    let mut entry = idt::Entry::<HandlerFunc>::missing();
                    entry.set_handler_fn(panicking_invalid_opcode_handler);
                    entry
                })
                .unwrap();
            idt_builder
                .set_stack_segment_fault_entry({
                    let mut entry = idt::Entry::<HandlerFuncWithErrCode>::missing();
                    entry.set_handler_fn(panicking_stack_segment_fault_handler);
                    entry
                })
                .unwrap();
            let spurious_interrupt_handler_index = set_spurious_interrupt_handler(
                &mut idt_builder,
                panicking_spurious_interrupt_handler,
            )
            .unwrap();
            let timer_interrupt_index = idt_builder
                .set_flexible_entry({
                    let mut entry = idt::Entry::missing();
                    entry.set_handler_fn(get_logging_timer_interrupt_handler(&LOCAL_APIC));
                    entry
                })
                .unwrap();
            let local_apic_error_interrupt_index = idt_builder
                .set_flexible_entry({
                    let mut entry = idt::Entry::<HandlerFunc>::missing();
                    entry.set_handler_fn(panicking_local_apic_error_interrupt_handler);
                    entry
                })
                .unwrap();

            tss.add_privilege_stack_table_entry({
                const STACK_SIZE: usize = 0x2000;
                static mut PRIV_TSS_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

                let stack_start = VirtAddr::from_ptr(unsafe {
                    #[allow(static_mut_refs)]
                    PRIV_TSS_STACK.as_mut_ptr()
                });
                stack_start + STACK_SIZE as u64
            })
            .unwrap();
            let keyboard =
                CoolKeyboardBuilder::set_interrupt(&mut idt_builder, &LOCAL_APIC).unwrap();
            let hpet = HpetBuilderStage0::default()
                .set_interrupt(&mut idt_builder)
                .unwrap();

            StaticStuff0 {
                tss: tss.get_tss(),
                idt_builder,
                spurious_interrupt_handler_index,
                timer_interrupt_index,
                local_apic_error_interrupt_index,
                keyboard,
                hpet,
            }
        })
        .unwrap();
    let static_stuff_1 = STATIC_STUFF_1
        .try_get_or_init(|| {
            let (tss_pointer, iopb) = static_stuff.tss.ready_to_activate();
            StaticStuff1 {
                gdt: Gdt::new(tss_pointer),
                iopb: Spinlock::new(iopb),
            }
        })
        .unwrap();
    static_stuff_1.gdt.init();
    static_stuff.idt_builder.init();

    let phys_mem_offset = VirtAddr::new(
        *boot_info
            .physical_memory_offset
            .as_ref()
            .expect("No physical memory mapped"),
    );
    let mut mapper = unsafe { memory::init(phys_mem_offset) };

    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(boot_info.memory_regions.deref_mut()) };

    let used_virt_mem_ranges = allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    let mapper = Arc::new(spin::Mutex::new(mapper));
    let virt_mem_tracker = Arc::new(spin::Mutex::new(used_virt_mem_ranges));
    let frame_allocator = Arc::new(spin::Mutex::new(frame_allocator));
    let phys_mapper = PhysMapper::new(
        phys_mem_offset.as_u64(),
        mapper.clone(),
        virt_mem_tracker.clone(),
        frame_allocator.clone(),
    );

    let acpi_tables = unsafe {
        acpi::init(
            boot_info.rsdp_addr.take().expect("No rsdp address!") as usize,
            phys_mapper.clone(),
        )
    }
    .expect("Error getting ACPI tables");

    replace_serial_logger_if_redirected(
        &acpi_tables,
        virt_mem_tracker.lock().deref_mut(),
        mapper.lock().deref_mut(),
        frame_allocator.lock().deref_mut(),
    )
    .unwrap();

    let apic = get_apic(&acpi_tables).unwrap();
    let local_apic = get_local_apic(
        &apic,
        &mut phys_mapper.clone(),
        static_stuff.spurious_interrupt_handler_index,
        static_stuff.timer_interrupt_index,
        static_stuff.local_apic_error_interrupt_index,
    )
    .unwrap();
    log::info!("Local APIC: {:#?}", local_apic);
    static_local_apic::store(UnsafeLocalApic(local_apic));

    #[allow(unused)]
    let mut io_apic = unsafe { get_io_apic(&apic, &mut phys_mapper.clone()) };

    let mut hpet_ref = hpet::init(&acpi_tables, phys_mapper.clone()).unwrap();
    static_stuff.hpet.configure_io_apic(
        hpet_ref.as_mut_ptr(),
        &mut io_apic,
        LOCAL_APIC.try_get().unwrap(),
    );
    HPET.try_init_once(|| RwLock::new(hpet_ref)).unwrap();

    let state = Arc::new(Mutex::new(None));
    let keyboard = static_stuff
        .keyboard
        .configure_io_apic(Arc::new(Mutex::new(io_apic)), state.clone());

    if let Some(ramdisk_addr) = boot_info.ramdisk_addr.as_ref() {
        let ram_disk = unsafe {
            slice::from_raw_parts(*ramdisk_addr as *const u8, boot_info.ramdisk_len as usize)
        };
        let ram_disk = postcard::from_bytes::<RamDisk>(ram_disk).unwrap();
        log::info!("Parsed ramdisk");
        // let user_space_mem_info = Arc::new(spin::Mutex::new(None));
        // init_syscalls(get_syscall_handler(
        //     frame_buffer,
        //     mapper.clone(),
        //     frame_allocator.clone(),
        //     keyboard,
        //     user_space_mem_info.clone(),
        //     state.clone(),
        // ));

        init_syscalls(set_syscall_handler_closure(Box::new(
            syscall_handler_closure(),
        )));

        spawn_task(
            ram_disk,
            frame_allocator.lock().deref_mut(),
            mapper.lock().deref_mut(),
        )
        .unwrap();

        log::info!("Tasks: {:#?}", TASKS.lock());

        run_tasks();
    }

    log::info!("There is no ramdisk so this kernel has nothing to do. Nothing. Interrupts aren't even enabled. This is the last message you will see. After that the CPU will be halted, and the computer will do nothing. You should probably turn off the computer now to save energy.");

    hlt_loop();
}
