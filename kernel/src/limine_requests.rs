use limine::modules::InternalModule;
use limine::mp::RequestFlags;
use limine::request::{
    BootloaderInfoRequest, DateAtBootRequest, ExecutableAddressRequest, FramebufferRequest,
    HhdmRequest, MemoryMapRequest, ModuleRequest, MpRequest, RequestsEndMarker,
    RequestsStartMarker, RsdpRequest, StackSizeRequest,
};
use limine::BaseRevision;

#[used]
#[unsafe(link_section = ".requests_start_marker")]
pub static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
pub static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static BOOT_TIME: DateAtBootRequest = DateAtBootRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static KERNEL_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static mut MP_REQUEST: MpRequest = MpRequest::new().with_flags(RequestFlags::X2APIC);

#[used]
#[unsafe(link_section = ".requests")]
pub static MODULE_REQUEST: ModuleRequest =
    ModuleRequest::new().with_internal_modules(&[&InternalModule::new().with_path(c"ram_disk")]);

#[used]
#[unsafe(link_section = ".requests")]
pub static LIMINE_BOOTLOADER_INFO_REQUEST: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static FRAME_BUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static STACK_SIZE_REQUEST: StackSizeRequest = StackSizeRequest::new().with_size(0x64_000);
