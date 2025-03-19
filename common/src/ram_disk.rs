use alloc::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::permissions::Permissions;

#[derive(Serialize, Deserialize)]
pub struct RamDisk<'a> {
    pub permissions: Permissions<'a>,
    #[serde(borrow)]
    pub elf: Cow<'a, [u8]>,
}
