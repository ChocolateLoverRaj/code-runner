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
#![feature(sync_unsafe_cell)]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// pub mod acpi;
pub mod acpi_handler_impl;
pub mod allocator;
pub mod apic;
pub mod colorful_logger;
pub mod combined_logger;
pub mod config;
pub mod context;
pub mod cpu_local;
pub mod cpu_local_data;
pub mod draw_rust;
pub mod dynamic_combined_logger;
pub mod embedded_graphics_writer;
pub mod ensure_mem_is_higher_half;
// pub mod execute_future;
pub mod frame_buffer;
pub mod get_rgb_color;
// pub mod get_total_memory;
pub mod hhdm_offset;
pub mod hlt_loop;
pub mod hpet_memory;
// pub mod init_cpus;
// pub mod init_idt_and_gdt;
pub mod iopb_size;
pub mod limine_requests;
pub mod log_boot_time;
pub mod log_bootloader_info;
pub mod log_cpu_info;
pub mod log_frame_buffer_info;
pub mod log_kernel_address;
// pub mod log_memory_usage;
pub mod log_phys_mem_regions;
pub mod log_ram_disk;
pub mod log_rsdp_addr;
pub mod log_sample_messages;
pub mod logger;
// pub mod logger_2;
pub mod logger_without_interrupts;
// pub mod map_local_xapic;
pub mod memory;
pub mod modules;
pub mod nmi_handler;
pub mod not_const_allocator;
pub mod panic_handler;
pub mod parse_ram_disk;
pub mod pic8259_interrupts;
// pub mod pt_allocator_2;
pub mod rsdp_addr;
// pub mod run_tasks;
pub mod set_color;
// pub mod spawn_task;
// pub mod spcr;
pub mod split_draw_target;
pub mod store_but_borrow_mut;
// pub mod terminate_current_task;
// pub mod syscall_enable_hpet;
// pub mod syscall_get_hpet_main_counter_period;
// pub mod syscall_handler_closure;
// pub mod syscall_handler_make_me_logger;
// pub mod syscall_hpet_read_main_counter_value;
// pub mod syscall_print_handler;
pub mod available_physical_frame_iterator;
// pub mod fault_handlers;
pub mod find_contiguous_unused_virtual_memory;
pub mod get_offset_page_table;
pub mod page_tables_recursive_iterator;
// pub mod tasks;
pub mod limine_frame_buffer_embedded_graphics;
pub mod logger_3;
pub mod test_allocator;
pub mod traverse_cr3;
pub mod user_space_state;
pub mod virt_addr_from_indexes;
pub mod virt_addr_to_number;
pub mod write_logger;
pub mod write_with_cr;

use core::{cell::RefCell, ops::DerefMut};

use acpi::spcr::Spcr;
use acpi_handler_impl::AcpiHandlerImpl;
use available_physical_frame_iterator::{
    AvailablePhysicalFrameIterator, AvailablePhysicalFrameIteratorFrameAllocator,
    AvailablePhysicalRegionsIterator,
};
use ensure_mem_is_higher_half::ensure_mem_is_higher_half;
use hhdm_offset::HhdmOffset;
use hlt_loop::hlt_loop;
// use init_cpus::init_cpus;
use iopb_size::IOPB_SIZE;
use limine_requests::{
    BASE_REVISION, FRAME_BUFFER_REQUEST, HHDM_REQUEST, KERNEL_ADDRESS_REQUEST, MEMORY_MAP_REQUEST,
    MODULE_REQUEST, MP_REQUEST, RSDP_REQUEST,
};
use log_boot_time::log_boot_time;
// use log_memory_usage::log_memory_usage;
#[allow(unused)]
use logger::init_logger_with_framebuffer;
use modules::idt::disable_pic8259::disable_pic8259;
use page_tables_recursive_iterator::PageTablesRecursiveIterator;
use rsdp_addr::RsdpAddr;
use test_allocator::test_allocator;
use util::static_allocator::StaticAllocator;
use x86_64::registers::control::Cr3;

#[export_name = "kernel_main"]
unsafe extern "C" fn kernel_main() -> ! {
    assert!(BASE_REVISION.is_supported());

    // init_logger_with_framebuffer(None);
    // log_sample_messages::log_sample_messages();
    let frame_buffer_response = FRAME_BUFFER_REQUEST.get_response();
    let hhdm_offset = HhdmOffset::try_from(&HHDM_REQUEST).unwrap();
    logger_3::init(frame_buffer_response, hhdm_offset);
    log::info!("Initialized logger to log on COM1 and the screen (if applicable)");

    let rsdp_addr = RsdpAddr::try_from(&RSDP_REQUEST).unwrap();

    let memory_map_response = MEMORY_MAP_REQUEST.get_response().unwrap();
    let frame_allocator = RefCell::new({
        let iterator = AvailablePhysicalFrameIterator::from(
            AvailablePhysicalRegionsIterator::from(memory_map_response),
        );
        unsafe { AvailablePhysicalFrameIteratorFrameAllocator::new(iterator) }
    });
    let acpi_tables = unsafe {
        acpi::AcpiTables::from_rsdp(
            AcpiHandlerImpl::new(hhdm_offset, &frame_allocator),
            u64::from(rsdp_addr) as usize,
        )
    }
    .unwrap();
    let spcr = acpi_tables.find_table::<Spcr>();
    if let Ok(spcr) = spcr {
        logger_3::init_spcr(&spcr, hhdm_offset, frame_allocator.borrow_mut().deref_mut());
    }
    log::info!("Initialized logger to log on SPCR instead of COM1 (if applicable)");
    disable_pic8259();

    log_bootloader_info::log_bootloader_info();
    log::info!("HHDM offset: {:?}", hhdm_offset);
    log_phys_mem_regions::log_phys_mem_regions();
    log_rsdp_addr::log_rsdp_addr(rsdp_addr);
    log_frame_buffer_info::log_frame_buffer_info(frame_buffer_response);

    let mp_response = unsafe {
        #[allow(static_mut_refs)]
        MP_REQUEST.get_response_mut().unwrap()
    };

    log_cpu_info::log_cpu_info(mp_response);

    let module_response = MODULE_REQUEST.get_response();
    log_ram_disk::log_ram_disk(module_response);

    let kernel_address_response = KERNEL_ADDRESS_REQUEST.get_response().unwrap();
    log_kernel_address::log_kernel_address(kernel_address_response);

    log_boot_time();

    // Safety: it has not been called before
    unsafe { allocator::init() };

    // Test assuming 400KiB is available for global allocation
    // test_allocator(0x100_000);

    todo!("Init CPUs");

    // init_cpus(mp_response, rsdp_addr, hhdm_offset, module_response)
}
