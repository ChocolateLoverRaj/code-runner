use alloc::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::permissions::MetaData;

#[derive(Serialize, Deserialize)]
pub struct RamDisk<'a> {
    pub meta_data: MetaData<'a>,
    #[serde(borrow)]
    pub elf: Cow<'a, [u8]>,
}
