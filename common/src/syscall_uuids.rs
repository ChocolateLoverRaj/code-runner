use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::{Uuid, uuid};

use crate::syscall_slice::SyscallSlice;

pub trait Syscall {
    const UUID: Uuid;

    type Input: Serialize + DeserializeOwned;
    type Output: Serialize + DeserializeOwned;

    fn serialize_to_input_with_uuid(input: &Self::Input) -> postcard::Result<[u64; 7]> {
        let mut arr: [u64; 7] = Default::default();
        let (a, b) = Self::UUID.as_u64_pair();
        arr[0] = a;
        arr[1] = b;
        postcard::to_slice(input, bytemuck::cast_slice_mut(&mut arr[2..])).unwrap();
        Ok(arr)
    }

    fn from_input_without_uuid(input: &[u64; 5]) -> postcard::Result<Self::Input> {
        let (syscall, _) = postcard::take_from_bytes(bytemuck::cast_slice(input))?;
        Ok(syscall)
    }

    fn serialize_output(output: &Self::Output) -> postcard::Result<[u64; 7]> {
        let mut arr: [u64; 7] = Default::default();
        postcard::to_slice(output, bytemuck::cast_slice_mut(&mut arr)).unwrap();
        Ok(arr)
    }

    fn deserialize_output(output: &[u64; 7]) -> postcard::Result<Self::Output> {
        let output = postcard::from_bytes(bytemuck::cast_slice(output))?;
        Ok(output)
    }
}

pub fn get_uuid(input: [u64; 7]) -> postcard::Result<(Uuid, [u64; 5])> {
    Ok((
        Uuid::from_u64_pair(input[0], input[1]),
        [input[2], input[3], input[4], input[5], input[6]],
    ))
}

// For making sure syscalls are working
#[derive(Debug, Serialize, Deserialize)]
pub struct SyscallTest;
impl Syscall for SyscallTest {
    const UUID: Uuid = uuid!("e38968bd-e8f7-4261-9628-3a9c365d1166");
    type Input = [u64; 5];
    type Output = [u64; 7];
}
impl SyscallTest {
    pub const TEST_INPUT: <Self as Syscall>::Input = [11, 22, 33, 44, 55];
    pub const TEST_OUTPUT: <Self as Syscall>::Output = [111, 222, 333, 444, 555, 666, 777];
}

// Core
#[derive(Debug, Serialize, Deserialize)]
pub struct SyscallExit;
impl Syscall for SyscallExit {
    const UUID: Uuid = uuid!("2cc55571-1c2e-4830-bff2-696eaf01ab50");
    type Input = ();
    type Output = ();
}

// Core, but I'm not sure if we really need this
#[derive(Debug, Serialize, Deserialize)]
pub struct SyscallExists {
    pub uuid_of_syscall_to_check_if_it_exists: Uuid,
}
impl Syscall for SyscallExists {
    const UUID: Uuid = uuid!("72ebbc54-e8d8-4608-88d2-3366ef474723");
    type Input = Uuid;
    type Output = bool;
}

// Logging
#[derive(Debug, Serialize, Deserialize)]
pub struct SyscallLog {
    pub message: SyscallSlice,
}
impl Syscall for SyscallLog {
    const UUID: Uuid = uuid!("64dcdda1-f673-43b9-97c2-5048cde056b0");
    type Input = SyscallSlice;
    type Output = ();
}
