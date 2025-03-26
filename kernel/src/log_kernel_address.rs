pub fn log_kernel_address(kernel_address_response: &limine::response::ExecutableAddressResponse) {
    log::info!(
        "Kernel at physical address: 0x{:X}, virtual address: 0x{:X}",
        kernel_address_response.physical_base(),
        kernel_address_response.virtual_base()
    );
}
