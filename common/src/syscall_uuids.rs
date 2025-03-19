use uuid::{Uuid, uuid};
use zerocopy::{FromBytes, IntoBytes, KnownLayout};

use crate::syscall_pointer::SyscallPointer;

// Core
pub const SYSCALL_EXIT: Uuid = uuid!("2cc55571-1c2e-4830-bff2-696eaf01ab50");
pub const SYSCALL_EXISTS: Uuid = uuid!("72ebbc54-e8d8-4608-88d2-3366ef474723");

// Logging (optional)
pub const SYSCALL_MAKE_ME_LOGGER: Uuid = uuid!("cf59d98c-5158-4a1e-8c74-822eeeb79ec9");

pub type SyscallMakeMeLoggerInput = SyscallPointer;

#[derive(Debug, FromBytes, IntoBytes, KnownLayout)]
#[repr(packed)]
pub struct SyscallMakeMeLoggerOutput {
    pub address: u64,
    pub event_uuid: u128,
}
