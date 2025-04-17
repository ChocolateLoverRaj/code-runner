use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::{Uuid, uuid};

use crate::syscall_slice::SyscallSlice;

pub trait Syscall {
    const UUID: Uuid;

    type Input: Serialize + DeserializeOwned;
    type Output: Serialize + DeserializeOwned;
}

pub fn get_uuid(input: [u64; 7]) -> postcard::Result<(Uuid, [u64; 5])> {
    Ok((
        Uuid::from_u64_pair(input[0], input[1]),
        [input[2], input[3], input[4], input[5], input[6]],
    ))
}

pub fn serialize_to_input_with_uuid<T: Syscall>(input: &T::Input) -> postcard::Result<[u64; 7]> {
    let mut arr: [u64; 7] = Default::default();
    let (a, b) = T::UUID.as_u64_pair();
    arr[0] = a;
    arr[1] = b;
    postcard::to_slice(input, bytemuck::cast_slice_mut(&mut arr[2..])).unwrap();
    Ok(arr)
}

pub fn from_input_without_uuid<T: Syscall>(input: &[u64; 5]) -> postcard::Result<T::Input> {
    let (syscall, _) = postcard::take_from_bytes(bytemuck::cast_slice(input))?;
    Ok(syscall)
}

pub fn serialize_output<T: Syscall>(output: &T::Output) -> postcard::Result<[u64; 7]> {
    let mut arr: [u64; 7] = Default::default();
    postcard::to_slice(output, bytemuck::cast_slice_mut(&mut arr)).unwrap();
    Ok(arr)
}

pub fn deserialize_output<T: Syscall>(output: &[u64; 7]) -> postcard::Result<T::Output> {
    let output = postcard::from_bytes(bytemuck::cast_slice(output))?;
    Ok(output)
}

// For making sure syscalls are working
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
pub struct SyscallExit;
impl Syscall for SyscallExit {
    const UUID: Uuid = uuid!("2cc55571-1c2e-4830-bff2-696eaf01ab50");
    type Input = ();
    type Output = ();
}

// Core, but I'm not sure if we really need this
pub struct SyscallExists;
impl Syscall for SyscallExists {
    const UUID: Uuid = uuid!("72ebbc54-e8d8-4608-88d2-3366ef474723");
    type Input = Uuid;
    type Output = bool;
}

// Logging
pub struct SyscallLog;
impl Syscall for SyscallLog {
    const UUID: Uuid = uuid!("64dcdda1-f673-43b9-97c2-5048cde056b0");
    type Input = SyscallSlice;
    type Output = ();
}

// Keyboard
#[derive(Debug, Serialize, Deserialize)]
pub enum IoPortAction {
    Take,
    Release,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SyscallTakeIoPortInput {
    pub port: u16,
    pub action: IoPortAction,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum SyscallTakeIoPortOutputError {
    /// The kernel is unable to give the user mode process access to the port because the I/O permission bitmap does not contain the port number
    OutOfIopb,
    /// The port is being used by another process
    InUse,
    /// You tried to release an I/O port that you didn't even take
    NotOwned,
}
pub struct SyscallTakeIoPort;
impl Syscall for SyscallTakeIoPort {
    const UUID: Uuid = uuid!("c22449e1-4d8f-44d5-909e-e8dda2e27f8d");
    type Input = SyscallTakeIoPortInput;
    type Output = Result<(), SyscallTakeIoPortOutputError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ListenAction {
    StartListening,
    StopListening,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum SyscallListenForKeyboardInterruptsOutputError {
    /// A different task is currently listening for keyboard interrupts
    InUse,
    /// Tried to stop listening when you aren't currently listening
    NotListening,
}
pub struct SyscallListenForKeyboardInterrupts;
impl Syscall for SyscallListenForKeyboardInterrupts {
    const UUID: Uuid = uuid!("4e1a4a2d-1b44-4374-9531-5ccf00f5c782");
    type Input = ListenAction;
    type Output = Result<(), SyscallListenForKeyboardInterruptsOutputError>;
}

pub struct SyscallWaitUntilEvent;
impl Syscall for SyscallWaitUntilEvent {
    const UUID: Uuid = uuid!("0018ef0c-e6fc-4531-991e-bc32dee29631");
    type Input = ();
    type Output = ();
}

// Screen
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ScreenInfo {
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bits_per_pixel: u16,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SyscallTakeScreenOutput {
    pub address: usize,
    pub info: ScreenInfo,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum SyscallTakeScreenError {
    /// The screen is being used
    InUse,
    /// There is no screen available
    NoScreenAvailable,
    /// This error will happen if the frame buffer is not contained in its own 4KiB physical frames.
    /// In this case, the kernel cannot give user mode access to the entire frame buffer because that would
    /// also give user space access to other physical memory which could be MMIO.
    WouldNotBeSecure,
}
pub struct SyscallTakeScreen;
impl Syscall for SyscallTakeScreen {
    const UUID: Uuid = uuid!("d5997323-ab9f-49f4-8927-6bdaf0948894");
    type Input = ();
    type Output = Result<SyscallTakeScreenOutput, SyscallTakeScreenError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SyscallReleaseScreenError {
    /// You do not currently own access to the screen
    NotOwned,
}
pub struct SyscallReleaseScreen;
impl Syscall for SyscallReleaseScreen {
    const UUID: Uuid = uuid!("86b7e3bc-7516-4242-bbab-1dcd92f68826");
    type Input = ();
    type Output = Result<(), SyscallReleaseScreenError>;
}
