use alloc::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::permissions::MetaData;

#[derive(Serialize, Deserialize)]
pub struct RamDisk<'a> {
    /// Meta-data for the kernel when spawning the process
    pub meta_data: MetaData<'a>,
    #[serde(borrow)]
    /// The ELF binary of the user space program
    pub elf: Cow<'a, [u8]>,
}
