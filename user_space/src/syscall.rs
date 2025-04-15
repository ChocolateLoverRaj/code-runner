use core::arch::asm;

use common::syscall_uuids::{
    deserialize_output, serialize_to_input_with_uuid, ListenAction, Syscall, SyscallExists,
    SyscallExit, SyscallListenForKeyboardInterrupts, SyscallLog, SyscallTakeIoPort,
    SyscallTakeIoPortInput, SyscallTest, SyscallWaitUntilEvent,
};
use uuid::Uuid;

/// # Safety
/// The inputs must be valid. Invalid inputs can lead to undefined behavior or the program being terminated.
pub unsafe fn syscall_internal(
    input0: u64,
    input1: u64,
    input2: u64,
    input3: u64,
    input4: u64,
    input5: u64,
    input6: u64,
) -> u64 {
    let return_value: u64;
    unsafe {
        asm!("\
            syscall
            ",
            in("rdi") input0,
            in("rsi") input1,
            in("rdx") input2,
            in("r10") input3,
            in("r8") input4,
            in("r9") input5,
            in("rax") input6,
            lateout("rax") return_value
        );
    }
    return_value
}

/// # Safety
/// The inputs must be valid. Invalid inputs can lead to undefined behavior or the program being terminated.
pub unsafe fn raw_syscall(inputs_and_ouputs: &mut [u64; 7]) {
    unsafe {
        asm!("\
            syscall
            ",
            inlateout("rdi") inputs_and_ouputs[0],
            inlateout("rsi") inputs_and_ouputs[1],
            inlateout("rdx") inputs_and_ouputs[2],
            inlateout("r10") inputs_and_ouputs[3],
            inlateout("r8") inputs_and_ouputs[4],
            inlateout("r9") inputs_and_ouputs[5],
            inlateout("rax") inputs_and_ouputs[6],
        );
    }
}

/// # Safety: Inputs must be correct
unsafe fn syscall<T: Syscall>(input: &T::Input) -> T::Output {
    let mut input_and_output = serialize_to_input_with_uuid::<T>(input).unwrap();
    unsafe { raw_syscall(&mut input_and_output) };
    let output = deserialize_output::<T>(&input_and_output).unwrap();
    output
}

pub fn syscall_test() {
    let output = unsafe { syscall::<SyscallTest>(&SyscallTest::TEST_INPUT) };
    assert_eq!(output, SyscallTest::TEST_OUTPUT);
}

pub fn syscall_exit() -> ! {
    unsafe { syscall::<SyscallExit>(&()) };
    unreachable!()
}

pub fn syscall_exists(uuid: &Uuid) -> bool {
    unsafe { syscall::<SyscallExists>(uuid) }
}

pub fn syscall_print(message: &str) {
    // Safety: safety rules for the &str are met
    unsafe { syscall::<SyscallLog>(&message.as_bytes().into()) }
}

pub fn syscall_take_io_port(
    input: &SyscallTakeIoPortInput,
) -> <SyscallTakeIoPort as Syscall>::Output {
    unsafe { syscall::<SyscallTakeIoPort>(input) }
}

pub fn syscall_listen_for_keyboard_interrupts(
    input: &ListenAction,
) -> <SyscallListenForKeyboardInterrupts as Syscall>::Output {
    unsafe { syscall::<SyscallListenForKeyboardInterrupts>(input) }
}

pub fn syscall_wait_until_event() -> <SyscallWaitUntilEvent as Syscall>::Output {
    unsafe { syscall::<SyscallWaitUntilEvent>(&()) }
}
