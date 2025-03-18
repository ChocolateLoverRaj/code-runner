use core::mem::MaybeUninit;

use uuid::{Uuid, uuid};
use zerocopy::{IntoBytes, KnownLayout, TryFromBytes};

use crate::syscall_slice::SyscallSlice;

pub const SYSCALL_EXIT: Uuid = uuid!("2cc55571-1c2e-4830-bff2-696eaf01ab50");
pub const SYSCALL_EXISTS: Uuid = uuid!("72ebbc54-e8d8-4608-88d2-3366ef474723");

pub const SYSCALL_ACCESS_COM1: Uuid = uuid!("6d515ed4-3853-4858-b10a-0a23e48d834a");

#[derive(Debug)]
pub struct SyscallAccessCom1Write {
    pub port_offset: u8,
    pub value: u8,
}

#[derive(Debug)]
pub struct SyscallAccessCom1Read {
    pub port_offset: u8,
    pub value: MaybeUninit<u8>,
}

#[derive(Debug, KnownLayout)]
pub enum SyscallAccessCom1IOItem {
    Write(SyscallAccessCom1Write),
    Read(SyscallAccessCom1Read),
}
pub type SyscallAccessCom1InputAndOutput = [SyscallAccessCom1IOItem];
pub type SyscallAccessCom1 = SyscallSlice;

#[derive(Debug, TryFromBytes, IntoBytes)]
#[repr(u8)]
pub enum SyscallAccessCom1Action {
    None,
    Read,
    Write,
}

#[derive(Debug, TryFromBytes, IntoBytes)]
pub struct SyscallAccessCom1Register {
    pub port_offset: u8,
    pub value: u8,
    pub action: SyscallAccessCom1Action,
}

pub type SyscallAccessCom1Memory = [SyscallAccessCom1Register; 8];
