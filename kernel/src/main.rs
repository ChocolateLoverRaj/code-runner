#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(int_roundings)]
#![feature(naked_functions)]
#![feature(pointer_is_aligned_to)]
#![feature(vec_push_within_capacity)]
#![feature(never_type)]
#![feature(fn_traits)]
#![feature(non_null_from_ref)]
#![feature(vec_into_raw_parts)]
#![feature(box_vec_non_null)]
#![feature(iter_collect_into)]
#![feature(sync_unsafe_cell)]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod acpi_handler_impl;
pub mod allocator;
pub mod apic;
pub mod combined_logger;
pub mod config;
pub mod context;
pub mod io_ports_lock;
// pub mod cpu_local;
pub mod check_pointer;
pub mod cpu_local_data;
pub mod dynamic_combined_logger;
pub mod ensure_mem_is_higher_half;
// pub mod execute_future;
// pub mod get_total_memory;
pub mod hhdm_offset;
pub mod hlt_loop;
pub mod hpet_memory;
pub mod init_cpus;
pub mod io_permission_bitmap;
pub mod iopb_size;
pub mod limine_requests;
pub mod log_boot_time;
pub mod log_bootloader_info;
pub mod log_cpu_info;
pub mod log_frame_buffer_info;
pub mod log_kernel_address;
pub mod mutex_without_interrupts;
pub mod syscalls;
// pub mod log_memory_usage;
pub mod log_phys_mem_regions;
pub mod log_ram_disk;
pub mod log_rsdp_addr;
pub mod log_sample_messages;
// pub mod logger_2;
pub mod logger_without_interrupts;
pub mod map_local_xapic;
pub mod modules;
pub mod nmi_handler;
pub mod not_const_allocator;
pub mod panic_handler;
pub mod parse_ram_disk;
pub mod pic8259_interrupts;
// pub mod pt_allocator_2;
pub mod physical_memory;
pub mod rsdp_addr;
pub mod run_tasks;
pub mod set_color;
pub mod spawn_task;
// pub mod spcr;
pub mod call_stack_iterator;
pub mod return_wait_until_event;
pub mod screen_lock;
pub mod split_draw_target;
pub mod store_but_borrow_mut;
pub mod terminate_current_task;
// pub mod syscall_enable_hpet;
// pub mod syscall_get_hpet_main_counter_period;
// pub mod syscall_handler_make_me_logger;
// pub mod syscall_hpet_read_main_counter_value;
// pub mod syscall_print_handler;
pub mod available_physical_frame_iterator;
pub mod backtrace_display;
pub mod boxed_stack;
pub mod bsp_init;
pub mod find_contiguous_unused_virtual_memory;
pub mod get_offset_page_table;
pub mod init_idt_and_gdt;
pub mod interrupt_handlers;
pub mod logger_3;
pub mod page_tables_recursive_iterator;
pub mod tasks;
pub mod test_allocator;
pub mod traverse_cr3;
pub mod user_space_state;
pub mod virt_addr_from_indexes;
pub mod virt_addr_to_number;
pub mod write_logger;
pub mod write_with_cr;

#[export_name = "kernel_main"]
unsafe extern "C" fn kernel_main() -> ! {
    // Safety: Only being called once as the first thing in the kernel.
    unsafe { bsp_init::init() };
}
