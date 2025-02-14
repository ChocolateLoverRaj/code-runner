/// The kernel gets the higher half of addresses, while the user space gets lower half
pub const KERNEL_VIRT_MEM_START: u64 = 0xFFFF_8000_0000_0000;
/// This will be used by memory mapped io like the frame buffer which doesn't need its own phys frames but needs space in the virt address space
pub const USER_SPACE_MMIO_START: u64 = KERNEL_VIRT_MEM_START - 0x40000000;
