use core::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};

use acpi::spcr::Spcr;

use crate::{
    acpi_handler_impl::AcpiHandlerImpl,
    allocator,
    available_physical_frame_iterator::{
        AvailablePhysicalFrameIterator, AvailablePhysicalFrameIteratorFrameAllocator,
        AvailablePhysicalRegionsIterator,
    },
    config::CONFIG,
    hhdm_offset::HhdmOffset,
    init_cpus::init_cpus,
    init_idt_and_gdt,
    limine_requests::{
        BASE_REVISION, FRAME_BUFFER_REQUEST, HHDM_REQUEST, KERNEL_ADDRESS_REQUEST,
        MEMORY_MAP_REQUEST, MODULE_REQUEST, MP_REQUEST, RSDP_REQUEST,
    },
    log_boot_time::log_boot_time,
    log_bootloader_info::{self},
    log_cpu_info::{self},
    log_frame_buffer_info,
    log_kernel_address::{self},
    log_phys_mem_regions::{self},
    log_ram_disk::{self},
    log_rsdp_addr::{self},
    log_sample_messages::log_sample_messages,
    logger_3,
    modules::idt::disable_pic8259::disable_pic8259,
    rsdp_addr::RsdpAddr,
};

/// The initialization of things that just need to be run on one CPU (the BSP) before running the every-CPU init
///
/// # Safety
/// This function must be called exactly once as the first thing in the kernel
pub unsafe fn init() -> ! {
    // This kernel should not be called on an unsupported version, but we stop just in case it is.
    assert!(BASE_REVISION.is_supported());

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
    if let Some(log_serial_config) = &CONFIG.kernel_log_serial {
        let spcr = acpi_tables.find_table::<Spcr>().ok();
        logger_3::init_spcr(
            spcr.as_ref().map(|spcr| spcr.deref()),
            log_serial_config,
            hhdm_offset,
            frame_allocator.borrow_mut().deref_mut(),
        );
    }
    log::info!("Checked for SPCR and initialized logger according to configuration");
    if CONFIG.kernel_log_sample_messages {
        log_sample_messages();
    }

    disable_pic8259();

    log_bootloader_info::log_bootloader_info();
    log::info!("HHDM offset: {:?}", hhdm_offset);
    log_phys_mem_regions::log_phys_mem_regions();
    log_rsdp_addr::log_rsdp_addr(rsdp_addr);
    log_frame_buffer_info::log_frame_buffer_info(frame_buffer_response);

    let mp_response = unsafe {
        #[allow(static_mut_refs)]
        MP_REQUEST.get_response().unwrap()
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

    init_idt_and_gdt::init_bsp(&acpi_tables, hhdm_offset, &frame_allocator);

    // Safety: Only being called once, after BSP init
    unsafe { init_cpus() }
}
