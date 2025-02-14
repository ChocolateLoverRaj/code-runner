use postcard::experimental::max_size::MaxSize;
use serde::{Serialize, de::DeserializeOwned};

// TODO: Figure out how to make sure max size <= 64 bits at compile time
pub trait SyscallOutput: Serialize + DeserializeOwned + MaxSize {
    fn to_syscall_output(&self) -> postcard::Result<u64> {
        let mut output = [u8::default(); size_of::<u64>()];
        postcard::to_slice(&self, &mut output)?;
        Ok(u64::from_ne_bytes(output))
    }

    fn from_syscall_output(syscall_output: u64) -> postcard::Result<Self> {
        postcard::from_bytes(&syscall_output.to_ne_bytes())
    }
}
