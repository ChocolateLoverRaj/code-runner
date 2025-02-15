use core::{arch::asm, mem::MaybeUninit};

use common::{
    syscall::Syscall,
    syscall_output::SyscallOutput,
    syscall_print::{SyscallPrintError, SyscallPrintOutput},
    syscall_start_recording_keyboard::SyscallStartRecordingKeyboardInput,
    syscall_take_frame_buffer::{
        TakeFrameBufferError, TakeFrameBufferOutput, TakeFrameBufferOutputData,
    },
};

/// # Safety
/// The inputs must be valid. Invalid inputs can lead to undefined behavior or the program being terminated.
unsafe fn syscall_internal(
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
            mov rdi, {0}
            mov rsi, {1}
            mov rdx, {2}
            mov r10, {3}
            mov r8,  {4}
            mov r9,  {5}
            mov rax, {6}
            syscall
            ",
            in(reg) input0,
            in(reg) input1,
            in(reg) input2,
            in(reg) input3,
            in(reg) input4,
            in(reg) input5,
            in(reg) input6,
            lateout("rax") return_value
        );
    }
    return_value
}

fn syscall(syscall: &Syscall) -> u64 {
    let [input0, input1, input2, input3, input4, input5, input6] =
        syscall.serialize_to_input().unwrap();
    // We know the inputs are valid
    unsafe { syscall_internal(input0, input1, input2, input3, input4, input5, input6) }
}

pub fn syscall_take_frame_buffer() -> Result<TakeFrameBufferOutputData, TakeFrameBufferError> {
    let mut output = MaybeUninit::<TakeFrameBufferOutputData>::uninit();
    TakeFrameBufferOutput::from_syscall_output(syscall(&Syscall::TakeFrameBuffer(
        output.as_mut_ptr().into(),
    )))
    .unwrap()
    .0?;
    // Because the kernel returned `Ok` we can trust the kernel to have initialized the pointer
    let dest = unsafe { output.assume_init() };
    Ok(dest)
}

pub fn syscall_print(string: &str) -> Result<(), SyscallPrintError> {
    SyscallPrintOutput::from_syscall_output(syscall(&Syscall::Print(string.as_bytes().into())))
        .unwrap()
        .0
}

pub fn syscall_exit() -> ! {
    syscall(&Syscall::Exit);
    unreachable!()
}

pub fn syscall_start_recording_keyboard(input: SyscallStartRecordingKeyboardInput) {
    syscall(&Syscall::StartRecordingKeyboard(input));
}

pub fn syscall_poll_keyboard(buffer: &mut [u8]) -> &mut [u8] {
    let count = syscall(&Syscall::PollKeyboard(buffer.into())) as usize;
    &mut buffer[..count]
}

pub fn syscall_block_until_event() {
    syscall(&Syscall::BlockUntilEvent);
}
