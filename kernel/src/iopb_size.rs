/// Max I/O address is a u16 and since there are 8 bits in a byte
// pub const IOPB_SIZE: usize = 2_usize.pow(16) / 8;

/// Right now we only care about allowing user space processes to access COM1, and this is the minimum size needed to control COM1 from the IOPB
pub const IOPB_SIZE: usize = (0x3F8 + 8) / 8;
