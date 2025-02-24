#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(int_roundings)]
#![feature(naked_functions)]
#![feature(pointer_is_aligned_to)]
#![feature(unsigned_is_multiple_of)]
#![feature(vec_push_within_capacity)]
#![feature(thread_local)]
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
pub mod logger;
pub mod logger_without_interrupts;
pub mod memory;
pub mod modules;
pub mod phys_mapper;
pub mod pic8259_interrupts;
pub mod serial_logger;
pub mod set_color;
pub mod spcr;
pub mod split_draw_target;
pub mod syscall_enable_hpet;
pub mod syscall_get_hpet_main_counter_period;
pub mod syscall_handler;
pub mod syscall_hpet_read_main_counter_value;
pub mod syscall_print_handler;
pub mod user_space_state;
pub mod virt_addr_from_indexes;
pub mod virt_mem_tracker;

use ::acpi::spcr::Spcr;
use alloc::sync::Arc;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use bootloader_x86_64_common::serial::SerialPort;
use common::mem::KERNEL_VIRT_MEM_START;
use conquer_once::noblock::OnceCell;
use cool_keyboard_interrupt_handler::CoolKeyboardBuilder;
use core::{
    fmt::Write,
    ops::{Deref, DerefMut},
    panic::PanicInfo,
    slice,
};
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
    syscall::{init_syscalls::init_syscalls, jmp_to_elf::jmp_to_elf},
    tss::TssBuilder,
    unsafe_local_apic::UnsafeLocalApic,
};
use phys_mapper::PhysMapper;
use spcr::get_16550_base_address;
use spin::{Mutex, RwLock};
use syscall_enable_hpet::syscall_enable_hpet;
use syscall_get_hpet_main_counter_period::syscall_get_hpet_main_counter_period;
use syscall_handler::get_syscall_handler;
use syscall_hpet_read_main_counter_value::syscall_hpet_read_main_counter_value;
use uart_16550_2::{DivisorLatchFlags, FifoCtrlFlags, ModemCtrlFlags};
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
    interrupts::disable();
    // TODO: Blue screen with a frowny face and a QR Code
    for _ in 0..60 {
        log::info!("hi");
    }
    log::error!("{}", info);
    hlt_loop()
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    // Use higher half for kernel to have space in the lower parts for ELFs
    config.mappings.dynamic_range_start = Some(KERNEL_VIRT_MEM_START);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

struct StaticStuff {
    tss: TaskStateSegment,
    idt_builder: IdtBuilder,
    spurious_interrupt_handler_index: u8,
    timer_interrupt_index: u8,
    local_apic_error_interrupt_index: u8,
    keyboard: CoolKeyboardBuilder,
    hpet: HpetBuilderStage1,
}

static STATIC_STUFF: OnceCell<StaticStuff> = OnceCell::uninit();
static GDT: OnceCell<Gdt> = OnceCell::uninit();

static HPET: OnceCell<RwLock<VolatileRef<HpetMemory>>> = OnceCell::uninit();

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let mut frame_buffer = boot_info.framebuffer.as_mut();
    if let Some(frame_buffer) = frame_buffer.as_mut() {
        frame_buffer.buffer_mut().fill(0);
    }
    init_logger_with_framebuffer(frame_buffer);

    log::info!(
        "Ramdisk len: {:?}. Ramdisk addr: {:?}",
        boot_info.ramdisk_len,
        boot_info.ramdisk_addr
    );
    let static_stuff = STATIC_STUFF
        .try_get_or_init(|| {
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
                    // entry.set_handler_fn(get_logging_timer_interrupt_handler(&LOCAL_APIC));
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

            StaticStuff {
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
    let gdt = GDT.try_get_or_init(|| Gdt::new(&static_stuff.tss)).unwrap();
    gdt.init();
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
        virt_mem_tracker,
        frame_allocator.clone(),
    );

    let acpi_tables = unsafe {
        acpi::init(
            boot_info.rsdp_addr.take().expect("No rsdp address!") as usize,
            phys_mapper.clone(),
        )
    }
    .expect("Error getting ACPI tables");

    // hlt_loop();

    let apic = get_apic(&acpi_tables).unwrap();
    let local_apic = get_local_apic(
        &apic,
        &mut phys_mapper.clone(),
        static_stuff.spurious_interrupt_handler_index,
        static_stuff.timer_interrupt_index,
        static_stuff.local_apic_error_interrupt_index,
    )
    .unwrap();
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

    // if let Some(ramdisk_addr) = boot_info.ramdisk_addr.as_ref() {
    //     let elf_bytes = unsafe {
    //         slice::from_raw_parts(*ramdisk_addr as *const u8, boot_info.ramdisk_len as usize)
    //     };
    //     log::info!("Entering ELF as user space");
    //     let user_space_mem_info = Arc::new(spin::Mutex::new(None));
    //     init_syscalls(get_syscall_handler(
    //         frame_buffer,
    //         mapper.clone(),
    //         frame_allocator.clone(),
    //         keyboard,
    //         user_space_mem_info.clone(),
    //         state.clone(),
    //     ));
    //     unsafe {
    //         jmp_to_elf(
    //             elf_bytes,
    //             mapper.clone(),
    //             frame_allocator.clone(),
    //             user_space_mem_info,
    //             state,
    //         )
    //     }
    //     .unwrap();
    // }

    log::info!("It did not crash");

    /// Blocks until the given amount of femtoseconds have passed
    pub fn spin_fs(duration_fs: u128) {
        // Doesn't hurt to enable if it's already enabled
        syscall_enable_hpet();
        let period_fs = syscall_get_hpet_main_counter_period();
        let counter_before = syscall_hpet_read_main_counter_value();
        loop {
            let counter_now = syscall_hpet_read_main_counter_value();
            let elapsed_fs = (counter_now - counter_before) as u128 * period_fs as u128;
            if elapsed_fs >= duration_fs {
                break;
            }
        }
    }

    spin_fs(3 * 10_u128.pow(15));
    let headers = acpi_tables.headers();

    headers.for_each(|header| log::info!("ACPI table header: {:?}", header.signature));
    log::info!("It did not crash 2");

    let r = acpi_tables.find_table::<Spcr>();
    if let Ok(r) = r {
        let r = r.deref();
        log::info!("Found SPCR: {:#?}", r);
    }

    let spcr_16550_base_phys_addr = acpi_tables
        .find_table::<Spcr>()
        .ok()
        .and_then(|spcr| get_16550_base_address(&spcr));
    log::info!(
        "SPCR 16550 Base (Physical) Address: {:?}",
        spcr_16550_base_phys_addr
    );
    if let Some(spcr_16550_base_phys_addr) = spcr_16550_base_phys_addr {
        let base_virt_addr =
            (phys_mem_offset + spcr_16550_base_phys_addr.as_u64()).as_mut_ptr::<u8>();
        unsafe {
            // outb(PORT + 1, 0x00); // Disable all interrupts
            base_virt_addr.offset(1).write_volatile(0x00);
            // outb(PORT + 3, 0x80); // Enable DLAB (set baud rate divisor)
            base_virt_addr.offset(3).write_volatile(0x80);
            // outb(PORT + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
            base_virt_addr.offset(0).write_volatile(0x01);
            // outb(PORT + 1, 0x00); //                  (hi byte)
            base_virt_addr.offset(1).write_volatile(0x00);
            // outb(PORT + 3, 0x03); // 8 bits, no parity, one stop bit
            base_virt_addr.offset(3).write_volatile(0x03);
            // outb(PORT + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
            base_virt_addr.offset(2).write_volatile(0xC7);
            // outb(PORT + 4, 0x0B); // IRQs enabled, RTS/DSR set
            base_virt_addr.offset(4).write_volatile(0x0B);
            // outb(PORT + 4, 0x1E); // Set in loopback mode, test the serial chip
            base_virt_addr.offset(4).write_volatile(0x1E);
            // outb(PORT + 0, 0xAE);
            base_virt_addr.offset(0).write_volatile(0xAE);

            // Check if serial is faulty (i.e: not same byte as sent)
            // if (inb(PORT + 0) != 0xAE) {
            //     return 1;
            // }
            if base_virt_addr.offset(0).read_volatile() != 0xAE {
                log::error!("Serial is faulty. Loopback mode set but received something else.");
                spin_fs(3 * 10_u128.pow(15));
            }

            // If serial is not faulty set it in normal operation mode
            // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
            // outb(PORT + 4, 0x0F);
            base_virt_addr.offset(4).write_volatile(0x0F);

            // Sending data
            // int is_transmit_empty() {
            //     return inb(PORT + 5) & 0x20;
            //  }
            let is_transmit_empty =
                || -> bool { base_virt_addr.offset(5).read_volatile() & 0x20 == 0 };

            // void write_serial(char a) {
            //     while (is_transmit_empty() == 0);

            //     outb(PORT,a);
            //  }
            let write_serial = |char: u8| {
                while is_transmit_empty() {
                    log::info!("Transmit is empty");
                    base_virt_addr.offset(0).write_volatile(char)
                }
            };

            loop {
                write_serial(b'a');
            }
        }

        // let mut serial_port =
        //     unsafe { uart_16550_2::MmioSerialPort::new(base_virt_addr.as_u64() as usize) };
        // log::info!("initializing serial port");
        // serial_port.init_values(
        //     DivisorLatchFlags::baud_115200(),
        //     FifoCtrlFlags::ENABLE,
        //     ModemCtrlFlags::DATA_TERMINAL_READY | ModemCtrlFlags::REQUEST_TO_SEND,
        // );
        // log::info!("Writing to serial port");
        // loop {
        //     serial_port
        //         .write_str("Hello from a computer which has a SPCR ACPI header!\r\n")
        //         .unwrap();
        //     log::info!("Wrote to the SPCR 16550 serial port");
        // }
    }

    hlt_loop();
}
