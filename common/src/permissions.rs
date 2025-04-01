use alloc::borrow::Cow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaData<'a> {
    /// The stack size that the user space process will be spawned with
    pub stack_size: u64,
    #[serde(borrow)]
    /// The permissions that the user space process will have
    pub permissions: Permissions<'a>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Permissions<'a> {
    /// A list of allowed x86 I/O ports
    #[serde(borrow)]
    pub ports: Cow<'a, [u16]>,
}
