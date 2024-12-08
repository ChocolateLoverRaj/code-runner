#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]

#[allow(unused)]
#[macro_use]
extern crate alloc;

pub mod acpi;
pub mod allocator;
pub mod apic;
pub mod change_stream;
pub mod colorful_logger;
pub mod combined_logger;
pub mod demo_async;
pub mod demo_async_keyboard_drop;
pub mod demo_async_rtc_drop;
pub mod draw_rust;
pub mod embedded_graphics_writer;
pub mod execute_future;
pub mod find_used_virt_addrs;
pub mod frame_buffer;
pub mod get_rgb_color;
pub mod hlt_loop;
pub mod insert;
pub mod keyboard_interrupt_mutex;
pub mod logger;
pub mod logger_without_interrupts;
pub mod memory;
pub mod modules;
pub mod phys_mapper;
pub mod pic8259_interrupts;
pub mod remove;
pub mod serial_logger;
pub mod set_color;
pub mod split_draw_target;
pub mod stream_with_initial;
pub mod virt_addr_from_indexes;
pub mod virt_mem_allocator;

use alloc::sync::Arc;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use conquer_once::noblock::OnceCell;
use core::{ops::DerefMut, panic::PanicInfo};
#[allow(unused)]
use demo_async::demo_async;
#[allow(unused)]
use demo_async_keyboard_drop::demo_async_keyboard_drop;
#[allow(unused)]
use demo_async_rtc_drop::demo_asyc_rtc_drop;
#[allow(unused)]
use draw_rust::draw_rust;
use execute_future::execute_future;
use futures_util::FutureExt;
use hlt_loop::hlt_loop;
#[allow(unused)]
use logger::init_logger_with_framebuffer;
use modules::{
    async_keyboard::AsyncKeyboardBuilder,
    async_rtc::AsyncRtcBuilder,
    double_fault_handler_entry::get_double_fault_entry,
    get_apic::get_apic,
    get_io_apic::get_io_apic,
    get_local_apic::get_local_apic,
    gtd::Gdt,
    idt::IdtBuilder,
    logging_breakpoint_handler::logging_breakpoint_handler,
    logging_timer_interrupt_handler::get_logging_timer_interrupt_handler,
    panicking_double_fault_handler::panicking_double_fault_handler,
    panicking_general_protection_fault_handler::panicking_general_protection_fault_handler,
    panicking_local_apic_error_interrupt_handler::panicking_local_apic_error_interrupt_handler,
    panicking_page_fault_handler::panicking_page_fault_handler,
    panicking_spurious_interrupt_handler::panicking_spurious_interrupt_handler,
    spurious_interrupt_handler::set_spurious_interrupt_handler,
    static_local_apic::{enable_and_store, get_getter},
    tss::TssBuilder,
};
use phys_mapper::PhysMapper;
use spin::Mutex;
use x86_64::{
    structures::{
        idt::{self, HandlerFunc, HandlerFuncWithErrCode, PageFaultHandlerFunc},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // TODO: Blue screen with a frowny face and a QR Code
    log::error!("{}", info);
    hlt_loop()
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

struct StaticStuff {
    tss: TaskStateSegment,
    idt_builder: IdtBuilder,
    spurious_interrupt_handler_index: u8,
    timer_interrupt_index: u8,
    local_apic_error_interrupt_index: u8,
    rtc_async: AsyncRtcBuilder,
    async_keyboard: AsyncKeyboardBuilder,
}

static STATIC_STUFF: OnceCell<StaticStuff> = OnceCell::uninit();

// static TSS: OnceCell<TaskStateSegment> = OnceCell::uninit();
static GDT: OnceCell<Gdt> = OnceCell::uninit();
// static IDT: OnceCell<IdtBuilder> = OnceCell::uninit();

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let mut frame_buffer = boot_info.framebuffer.as_mut();
    let frame_buffer_for_drawing = frame_buffer.take().unwrap();
    init_logger_with_framebuffer(frame_buffer);
    let static_stuff = STATIC_STUFF
        .try_get_or_init(|| {
            let mut tss = TssBuilder::new();
            // let gdt = Gdt::new(&tss);
            let mut idt_builder = IdtBuilder::new();
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
            let spurious_interrupt_handler_index = set_spurious_interrupt_handler(
                &mut idt_builder,
                panicking_spurious_interrupt_handler,
            )
            .unwrap();
            let timer_interrupt_handler = get_logging_timer_interrupt_handler(get_getter());
            let timer_interrupt_index = idt_builder
                .set_flexible_entry({
                    let mut entry = idt::Entry::missing();
                    entry.set_handler_fn(timer_interrupt_handler);
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
            let rtc_async = AsyncRtcBuilder::set_interrupt(&mut idt_builder).unwrap();
            let async_keyboard = AsyncKeyboardBuilder::set_interrupt(&mut idt_builder).unwrap();

            StaticStuff {
                tss: tss.get_tss(),
                idt_builder,
                spurious_interrupt_handler_index,
                timer_interrupt_index,
                local_apic_error_interrupt_index,
                rtc_async,
                async_keyboard,
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
    let virt_mem_allocator = Arc::new(spin::Mutex::new(used_virt_mem_ranges.to_vec()));
    let frame_allocator = Arc::new(spin::Mutex::new(frame_allocator));
    let phys_mapper = PhysMapper::new(mapper, virt_mem_allocator, frame_allocator);
    let acpi_tables = unsafe {
        acpi::init(
            boot_info.rsdp_addr.take().expect("No rsdp address!") as usize,
            phys_mapper.clone(),
        )
    }
    .expect("Error getting ACPI tables");
    let apic = get_apic(&acpi_tables).unwrap();
    let local_apic = get_local_apic(
        &apic,
        &mut phys_mapper.clone(),
        static_stuff.spurious_interrupt_handler_index,
        static_stuff.timer_interrupt_index,
        static_stuff.local_apic_error_interrupt_index,
    )
    .unwrap();
    enable_and_store(local_apic);

    let mut io_apic = get_io_apic(&apic, &mut phys_mapper.clone());

    #[allow(unused)]
    let mut async_rtc = static_stuff
        .rtc_async
        .configure_io_apic(&mut io_apic, get_getter());
    let io_apic = Arc::new(Mutex::new(io_apic));
    #[allow(unused)]
    let mut async_keyboard =
        static_stuff
            .async_keyboard
            .configure_io_apic(io_apic, get_getter(), 100);
    x86_64::instructions::interrupts::enable();

    execute_future(
        async move {
            // demo_async(&mut async_keyboard, &mut async_rtc).await;
            // demo_async_keyboard_drop(&mut async_keyboard).await;
            // demo_asyc_rtc_drop(&mut async_rtc).await;
        }
        .boxed(),
    );

    log::info!("It did not crash");

    draw_rust(frame_buffer_for_drawing);

    hlt_loop();
}
