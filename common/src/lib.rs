#![cfg_attr(not(test), no_std)]

use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, MaxSize, PartialEq, Eq)]
pub enum Syscall {
    Print(u64),
}

impl Syscall {
    pub fn serialize_to_input(&self) -> postcard::Result<[u64; 7]> {
        let mut arr: [u64; 7] = Default::default();
        postcard::to_slice(self, bytemuck::cast_slice_mut(&mut arr)).unwrap();
        Ok(arr)
    }

    pub fn deserialize_from_input(syscall: [u64; 7]) -> postcard::Result<Self> {
        let (syscall, _) = postcard::take_from_bytes(bytemuck::cast_slice(&syscall))?;
        Ok(syscall)
    }
}

#[cfg(test)]
mod test {
    use postcard::experimental::max_size::MaxSize;

    use crate::Syscall;

    #[test]
    fn fits_in_input() {
        assert_eq!(Syscall::POSTCARD_MAX_SIZE <= size_of::<u64>() * 7, true)
    }
}

#[cfg(test)]
mod test2 {
    use crate::Syscall;

    #[test]
    fn serialize_and_deserialize() {
        let syscall = Syscall::Print(0x1000);
        let serialized_syscall = syscall.serialize_to_input().unwrap();
        let deserialiezd_syscall = Syscall::deserialize_from_input(serialized_syscall).unwrap();
        assert_eq!(deserialiezd_syscall, syscall);
    }
}
