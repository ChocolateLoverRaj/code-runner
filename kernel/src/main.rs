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
#![feature(non_null_from_ref)]
#![feature(vec_into_raw_parts)]
#![feature(box_vec_non_null)]
#![feature(iter_collect_into)]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod acpi;
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
pub mod ensure_mem_is_higher_half;
pub mod enter_user_mode;
pub mod execute_future;
pub mod find_used_virt_addrs;
pub mod frame_buffer;
pub mod get_rgb_color;
pub mod get_total_memory;
pub mod hlt_loop;
pub mod hpet;
pub mod hpet_memory;
pub mod insert;
pub mod iopb_size;
pub mod limine_requests;
pub mod log_boot_time;
pub mod log_bootloader_info;
pub mod log_cpu_info;
pub mod log_frame_buffer_info;
pub mod log_phys_mem_regions;
pub mod log_rsdp_addr;
pub mod log_sample_messages;
pub mod logger;
pub mod logger_without_interrupts;
pub mod memory;
pub mod modules;
pub mod not_const_allocator;
pub mod panic_handler;
pub mod phys_mapper;
pub mod pic8259_interrupts;
pub mod pt_allocator_2;
pub mod run_tasks;
pub mod set_color;
pub mod spawn_task;
pub mod spcr;
pub mod split_draw_target;
pub mod store_but_borrow_mut;
pub mod syscall_enable_hpet;
pub mod syscall_get_hpet_main_counter_period;
pub mod syscall_handler_closure;
pub mod syscall_handler_make_me_logger;
pub mod syscall_hpet_read_main_counter_value;
pub mod syscall_print_handler;
pub mod tasks;
pub mod test_allocator;
pub mod traverse_cr3;
pub mod user_space_state;
pub mod virt_addr_from_indexes;
pub mod virt_addr_to_number;
pub mod virt_mem_tracker;
pub mod write_logger;
pub mod write_with_cr;

use conquer_once::noblock::OnceCell;
use cool_keyboard_interrupt_handler::CoolKeyboardBuilder;
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
use ensure_mem_is_higher_half::ensure_mem_is_higher_half;
use get_total_memory::{
    get_acpi_reclaimable_memory, get_bootloader_reclaimable_memory, get_kernel_memory,
    get_total_memory,
};
use hlt_loop::hlt_loop;
use hpet::{HpetBuilderStage0, HpetBuilderStage1};
use hpet_memory::HpetMemory;
use iopb_size::IOPB_SIZE;
use limine::{self, framebuffer::MemoryModel, memory_map::EntryType};
use limine_requests::{
    BASE_REVISION, FRAME_BUFFER_REQUEST, HHDM_REQUEST, KERNEL_ADDRESS_REQUEST,
    LIMINE_BOOTLOADER_INFO_REQUEST, MEMORY_MAP_REQUEST, MODULE_REQUEST, MP_REQUEST, RSDP_REQUEST,
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
use test_allocator::test_allocator;
use util::init_later::InitLater;
use volatile::VolatileRef;
use x86_64::{
    instructions::interrupts,
    registers::control::{Cr3, Cr3Flags},
    structures::{
        idt::{self, HandlerFunc, HandlerFuncWithErrCode, PageFaultHandlerFunc},
        paging::PhysFrame,
        tss::TaskStateSegment,
    },
    PhysAddr, VirtAddr,
};

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
    // log_sample_messages::log_sample_messages();
    log_bootloader_info::log_bootloader_info();
    log_phys_mem_regions::log_phys_mem_regions();
    log_rsdp_addr::log_rsdp_addr();
    log_frame_buffer_info::log_frame_buffer_info();

    let mp_response = unsafe {
        #[allow(static_mut_refs)]
        MP_REQUEST.get_response_mut().unwrap()
    };

    log_cpu_info::log_cpu_info(mp_response);

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

    let kernel_address_response = KERNEL_ADDRESS_REQUEST.get_response().unwrap();
    log::info!(
        "Kernel at physical address: 0x{:X}, virtual address: 0x{:X}",
        kernel_address_response.physical_base(),
        kernel_address_response.virtual_base()
    );
    // let kernel_file_response = KERNEL_FILE_REQUEST.get_response().unwrap();
    // log::info!(
    //     "Kernel file at address: {:?} with len 0x{:X}",
    //     kernel_file_response.file().addr(),
    //     kernel_file_response.file().size()
    // );

    ensure_mem_is_higher_half(hhdm_offset);

    log_boot_time();

    let memory_map_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    pt_allocator_2::init::init(memory_map_response, hhdm_offset);

    log::info!(
        "Total memory: 0x{:X}",
        get_total_memory(memory_map_response)
    );
    log::info!(
        "Bootloader reclaimable memory: 0x{:X}",
        get_bootloader_reclaimable_memory(memory_map_response)
    );
    log::info!(
        "ACPI reclaimable memory: 0x{:X}",
        get_acpi_reclaimable_memory(memory_map_response)
    );
    log::info!("Used memory: 0x{:X}", get_kernel_memory());

    // Test assuming 100MiB is available for dynamic allocation
    // test_allocator(0x6400000);

    // let phys_mapper = PhysMapper::new(hhdm_offset, mapper, virt_mem_tracker, frame_allocator)

    // let acpi_tables = unsafe {
    //     acpi::init(
    //         rsdp ,
    //         phys_mapper.clone(),
    //     )
    // }

    let cr3_val = {
        let (frame, flags) = Cr3::read();
        frame.start_address().as_u64() | flags.bits()
    };
    mp_response.cpus_mut().iter_mut().for_each(|cpu| {
        cpu.extra = cr3_val;
        cpu.goto_address.write(cpu_init);
    });

    let current_cpu = mp_response
        .cpus()
        .iter()
        .find(|cpu| cpu.lapic_id == mp_response.bsp_lapic_id())
        .unwrap();
    unsafe { cpu_init(current_cpu) }
}

unsafe extern "C" fn cpu_init(cpu: &limine::mp::Cpu) -> ! {
    let previous_cr3 = Cr3::read();
    let flags = Cr3Flags::from_bits_truncate(cpu.extra);
    let cr3_frame = PhysFrame::containing_address(PhysAddr::new(cpu.extra));
    unsafe { Cr3::write(cr3_frame, flags) };
    log::info!(
        "Hello from CPU: {:?}. Previous cr3: {:?}, current cr3: {:?}",
        cpu.id,
        previous_cr3.0,
        cr3_frame
    );
    hlt_loop()
}
