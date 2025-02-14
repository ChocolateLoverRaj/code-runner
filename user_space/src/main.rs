#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::{arch::asm, fmt::Write, mem::MaybeUninit, panic::PanicInfo};

use common::{
    syscall::{Syscall, TakeFrameBufferError, TakeFrameBufferOutput, TakeFrameBufferOutputData},
    syscall_slice::SyscallSlice,
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

fn syscall_take_frame_buffer() -> Result<TakeFrameBufferOutputData, TakeFrameBufferError> {
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

#[unsafe(no_mangle)]
extern "C" fn _start() {
    let mut count: u64 = 0;
    let mut frame_buffer = syscall_take_frame_buffer().unwrap();
    loop {
        frame_buffer
            .buffer_mut()
            .fill((count % u8::MAX as u64) as u8);
        let mut message = heapless::String::<100>::new();
        message
            .write_fmt(format_args!("Hello from user space. Counter: {}", count))
            .unwrap();
        syscall(&Syscall::Print(SyscallSlice::from_slice(
            message.as_bytes(),
        )));
        count += 1;
    }
}

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    // TODO: Nicer error display
    let mut message = heapless::String::<100>::new();
    message.write_fmt(format_args!("{}", panic_info)).unwrap();
    syscall(&Syscall::Print(SyscallSlice::from_slice(
        message.as_bytes(),
    )));
    // TODO: Exit process
    loop {}
}
