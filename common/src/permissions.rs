use alloc::borrow::Cow;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Permissions<'a> {
    #[serde(borrow)]
    pub ports: Cow<'a, [u16]>,
}
