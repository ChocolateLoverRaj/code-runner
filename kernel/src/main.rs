#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[allow(unused)]
#[macro_use]
extern crate alloc;

pub mod acpi;
pub mod allocator;
pub mod apic;
pub mod change_stream;
pub mod colorful_logger;
pub mod combined_logger;
pub mod draw_rust;
pub mod embedded_graphics_writer;
pub mod find_used_virt_addrs;
pub mod frame_buffer;
pub mod get_rgb_color;
pub mod hlt_loop;
pub mod insert;
pub mod interrupts;
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
pub mod task;
pub mod virt_addr_from_indexes;
pub mod virt_mem_allocator;

use alloc::sync::Arc;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use change_stream::StreamChanges;
use chrono::DateTime;
use conquer_once::noblock::OnceCell;
use core::{ops::DerefMut, panic::PanicInfo};
use futures_util::{future::join, FutureExt, StreamExt};
use hlt_loop::hlt_loop;
use logger::init_logger_with_framebuffer;
use modules::{
    double_fault_handler_entry::get_double_fault_entry, gtd::Gdt, idt::IdtBuilder,
    logging_breakpoint_handler::logging_breakpoint_handler,
    panicking_double_fault_handler::panicking_double_fault_handler,
    panicking_general_protection_fault_handler::panicking_general_protection_fault_handler,
    panicking_page_fault_handler::panicking_page_fault_handler,
    panicking_spurious_interrupt_handler::panicking_spurious_interrupt_handler,
    spurious_interrupt_handler::set_spurious_interrupt_handler, tss::TssBuilder,
};
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};
use phys_mapper::PhysMapper;
use stream_with_initial::StreamWithInitial;
use task::{execute_future::execute_future, keyboard::ScancodeStream, rtc::RtcStream};
use x86_64::{
    structures::{
        idt::{self, HandlerFunc, HandlerFuncWithErrCode, PageFaultHandlerFunc},
        tss::TaskStateSegment,
    },
    VirtAddr,
};
use x86_rtc::{interrupts::DividerValue, Rtc};

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
}

static STATIC_STUFF: OnceCell<StaticStuff> = OnceCell::uninit();

// static TSS: OnceCell<TaskStateSegment> = OnceCell::uninit();
static GDT: OnceCell<Gdt> = OnceCell::uninit();
// static IDT: OnceCell<IdtBuilder> = OnceCell::uninit();

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // let frame_buffer = boot_info.framebuffer.as_mut().unwrap();
    // draw_rust::draw_rust(frame_buffer);
    init_logger_with_framebuffer(&mut boot_info.framebuffer);
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

            StaticStuff {
                tss: tss.get_tss(),
                idt_builder,
                spurious_interrupt_handler_index,
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
    unsafe {
        apic::init(
            phys_mapper,
            acpi_tables,
            static_stuff.spurious_interrupt_handler_index,
        )
    }
    .unwrap();

    let rtc = Arc::new(Rtc::new());

    execute_future(
        join(
            async {
                let mut scancodes = ScancodeStream::new().unwrap();
                let mut keyboard = Keyboard::new(
                    ScancodeSet1::new(),
                    layouts::Us104Key,
                    HandleControl::Ignore,
                );

                while let Some(scancode) = scancodes.next().await {
                    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                        log::info!("{key_event:?}");
                    }
                }
            },
            RtcStream::new(DividerValue::new(15).unwrap())
                .unwrap()
                .with_initial(())
                .map(move |()| rtc.get_unix_timestamp())
                .changes()
                .for_each(|rtc_unix_timestamp| async move {
                    let now = DateTime::from_timestamp(rtc_unix_timestamp as i64, 0);
                    match now {
                        Some(now) => {
                            let now = now.to_rfc2822();
                            log::info!("Time (in UTC): {now}");
                        }
                        None => {
                            log::warn!("Invalid RTC time: {rtc_unix_timestamp}");
                        }
                    }
                }),
        )
        .boxed(),
    );

    log::info!("It did not crash");

    hlt_loop();
}
