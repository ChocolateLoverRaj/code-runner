use alloc::borrow::Cow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaData<'a> {
    pub stack_size: u64,
    #[serde(borrow)]
    pub permissions: Permissions<'a>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Permissions<'a> {
    #[serde(borrow)]
    pub ports: Cow<'a, [u16]>,
}
