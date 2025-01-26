#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(int_roundings)]
#![feature(naked_functions)]

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
pub mod demo_maze_roller_game;
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

use alloc::{alloc::alloc, sync::Arc};
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use conquer_once::noblock::OnceCell;
use core::{
    alloc::Layout,
    arch::{asm, naked_asm},
    cell::UnsafeCell,
    ops::DerefMut,
    panic::PanicInfo,
};
#[allow(unused)]
use demo_async::demo_async;
#[allow(unused)]
use demo_async_keyboard_drop::demo_async_keyboard_drop;
#[allow(unused)]
use demo_async_rtc_drop::demo_asyc_rtc_drop;
#[allow(unused)]
use demo_maze_roller_game::demo_maze_roller_game;
#[allow(unused)]
use draw_rust::draw_rust;
use hlt_loop::hlt_loop;
#[allow(unused)]
use logger::init_logger_with_framebuffer;
use memory::get_active_level_4_table;
use modules::{
    double_fault_handler_entry::get_double_fault_entry, gdt::Gdt, idt::IdtBuilder,
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
    spurious_interrupt_handler::set_spurious_interrupt_handler, static_local_apic::get_getter,
    tss::TssBuilder,
};
use phys_mapper::PhysMapper;
use x86_64::{
    instructions::tlb::flush_all,
    registers::{control::Cr3, model_specific::Msr},
    structures::{
        idt::{self, HandlerFunc, HandlerFuncWithErrCode, PageFaultHandlerFunc},
        paging::{frame::PhysFrameRange, PageSize, PageTable, PageTableFlags, PhysFrame, Size4KiB},
        tss::TaskStateSegment,
    },
    PhysAddr, PrivilegeLevel, VirtAddr,
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
    // config.mappings.dynamic_range_start = Some(0xFFFF_8000_0000_0000);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

struct StaticStuff {
    tss: TaskStateSegment,
    idt_builder: IdtBuilder,
    spurious_interrupt_handler_index: u8,
    timer_interrupt_index: u8,
    local_apic_error_interrupt_index: u8,
}

static STATIC_STUFF: OnceCell<StaticStuff> = OnceCell::uninit();

static GDT: OnceCell<Gdt> = OnceCell::uninit();

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let frame_buffer = boot_info.framebuffer.as_mut();
    init_logger_with_framebuffer(frame_buffer);
    let static_stuff = STATIC_STUFF
        .try_get_or_init(|| {
            let mut tss = TssBuilder::new();
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
                    log::info!(
                        "Invalid opcode handler: {:?}",
                        panicking_invalid_opcode_handler as *const ()
                    );
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

            tss.add_privilege_stack_table_entry({
                const STACK_SIZE: usize = 0x2000;
                const PRIV_TSS_STACK: UnsafeCell<[u8; STACK_SIZE]> =
                    UnsafeCell::new([0; STACK_SIZE]);

                let stack_start = VirtAddr::from_ptr(PRIV_TSS_STACK.get());
                let stack_end = stack_start + STACK_SIZE as u64;
                stack_end
            })
            .unwrap();

            StaticStuff {
                tss: tss.get_tss(),
                idt_builder,
                spurious_interrupt_handler_index,
                timer_interrupt_index,
                local_apic_error_interrupt_index,
            }
        })
        .unwrap();
    let gdt = GDT.try_get_or_init(|| Gdt::new(&static_stuff.tss)).unwrap();
    gdt.init();
    static_stuff.idt_builder.init();
    unsafe { init_syscalls() };

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

    x86_64::instructions::interrupts::enable();

    let cr3 = Cr3::read().0;
    log::info!("Cr3 (active L4 page table): {:?}", cr3);

    let translate_virt_to_phys = |virt_addr: VirtAddr| -> PhysAddr {
        let l4: &PageTable = unsafe { get_active_level_4_table(phys_mem_offset) };
        let l3_addr = l4[virt_addr.p4_index()].addr();
        let l3 = unsafe {
            &*(VirtAddr::new(l3_addr.as_u64() + phys_mem_offset.as_u64()).as_ptr::<PageTable>())
        };
        let l2_addr = l3[virt_addr.p3_index()].addr();
        let l2 = unsafe {
            &*(VirtAddr::new(l2_addr.as_u64() + phys_mem_offset.as_u64()).as_ptr::<PageTable>())
        };
        let l1_addr = l2[virt_addr.p2_index()].addr();
        let l1 = unsafe {
            &*(VirtAddr::new(l1_addr.as_u64() + phys_mem_offset.as_u64()).as_ptr::<PageTable>())
        };
        let phys_addr = l1[virt_addr.p1_index()].addr() + u64::from(virt_addr.page_offset());
        phys_addr
    };

    let userspace_fn_in_kernel = VirtAddr::from_ptr(userspace_prog_1 as *const ());
    log::info!(
        "Userspace fn address (in kernel): {:?}",
        userspace_fn_in_kernel
    );
    let userspace_fn_phys = translate_virt_to_phys(userspace_fn_in_kernel);
    log::info!("Userspace fn phys address: {:?}", userspace_fn_phys);
    let userspace_fn_frames = {
        let start_frame = PhysFrame::<Size4KiB>::containing_address(userspace_fn_phys);
        PhysFrameRange {
            start: start_frame,
            // Map 2 in case the fn takes up >1
            end: start_frame + 2,
        }
    };
    let userspace_fn_in_userspace = unsafe {
        phys_mapper.map_to_phys(
            userspace_fn_frames,
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE,
        )
    };
    log::info!(
        "Mapped userspace fn to page range: {:?}",
        userspace_fn_in_userspace
    );
    assert_eq!(
        translate_virt_to_phys(userspace_fn_in_userspace.start.start_address()),
        userspace_fn_phys.align_down(Size4KiB::SIZE)
    );

    let stack_size = 0x1000;
    let stack_space_virt = VirtAddr::from_ptr(unsafe {
        alloc(Layout::from_size_align(stack_size, Size4KiB::SIZE as usize).unwrap())
    });
    let stack_space_phys = translate_virt_to_phys(stack_space_virt);
    log::info!("mapping to phys range");
    let stack_in_userspace = unsafe {
        phys_mapper.map_to_phys(
            PhysFrameRange {
                start: PhysFrame::from_start_address(stack_space_phys).unwrap(),
                end: PhysFrame::containing_address(stack_space_phys + stack_size as u64) + 1,
            },
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        )
    };
    log::info!(
        "Mapped userspace stack to page range: {:?}",
        stack_in_userspace
    );
    assert_eq!(
        translate_virt_to_phys(stack_in_userspace.start.start_address()),
        stack_space_phys
    );

    let code = userspace_fn_in_userspace.start.start_address()
        + userspace_fn_in_kernel.page_offset().into();
    log::info!(
        "a: {:?} b: {:?}",
        userspace_fn_in_kernel.page_offset(),
        userspace_fn_in_kernel - userspace_fn_in_kernel.align_down(Size4KiB::SIZE),
    );

    log::info!(
        "Syscall handler addr: {:?}",
        VirtAddr::from_ptr(handle_syscall as *const ())
    );

    log::info!("Jumping to code address: {:?}", code);
    unsafe {
        jmp_to_usermode(
            gdt,
            code,
            stack_in_userspace.start.start_address() + stack_size as u64,
        )
    };

    log::info!("It did not crash");

    // draw_rust(frame_buffer_for_drawing);

    hlt_loop();
}

unsafe fn userspace_prog_1() {
    // asm!("nop");
    // asm!("nop");
    asm!(
        "\
        nop
        nop
        syscall
        nop
        nop",
        options(nostack, preserves_flags)
    );
    // asm!("nop");
    // asm!("nop");
}

pub unsafe fn jmp_to_usermode(gdt: &Gdt, code: VirtAddr, stack_end: VirtAddr) {
    let cs_idx = {
        let mut code_selector = gdt.user_code_selector.clone();
        code_selector.0 |= PrivilegeLevel::Ring3 as u16;
        code_selector.0
    };
    let ds_idx = {
        let mut data_selector = gdt.user_data_selector.clone();
        data_selector.0 |= PrivilegeLevel::Ring3 as u16;
        // DS::set_reg(data_selector.clone());
        data_selector.0
    };
    flush_all();
    asm!("\
    push rax   // stack segment
    push rsi   // rsp
    push 0x200 // rflags (only interrupt bit set)
    push rdx   // code segment
    push rdi   // ret to virtual addr
    iretq",
    in("rdi") code.as_u64(), in("rsi") stack_end.as_u64(), in("dx") cs_idx, in("ax") ds_idx);
}

const MSR_STAR: usize = 0xc0000081;
const MSR_LSTAR: usize = 0xc0000082;
const MSR_FMASK: usize = 0xc0000084;

pub unsafe fn init_syscalls() {
    let handler_addr = handle_syscall as *const () as u64;
    // let handler_addr = 0;

    log::info!("Handler address: {:?}", VirtAddr::new(handler_addr));

    let mut efer = Msr::new(0xC0000080);
    let mut value = efer.read();
    value |= 0b1;
    efer.write(value);

    // clear Interrupt flag on syscall with AMD's MSR_FSTAR register
    asm!("\
    xor rdx, rdx
    mov rax, 0x200
    wrmsr", in("rcx") MSR_FMASK, out("rdx") _);
    // write handler address to AMD's MSR_LSTAR register
    asm!("\
    mov rdx, rax
    shr rdx, 32
    wrmsr", in("rax") handler_addr, in("rcx") MSR_LSTAR, out("rdx") _);
    // write segments to use on syscall/sysret to AMD'S MSR_STAR register
    asm!("\
    xor rax, rax
    mov rdx, 0x230008 // use seg selectors 8, 16 for syscall and 43, 51 for sysret
    wrmsr", in("rcx") MSR_STAR, out("rax") _, out("rdx") _);
}

#[naked]
extern "C" fn handle_syscall() {
    unsafe {
        naked_asm!(
            "\
            nop
            nop
            nop
            sysretq
        "
        );
    };
}
