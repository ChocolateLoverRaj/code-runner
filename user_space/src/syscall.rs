use core::{arch::asm, mem::MaybeUninit};

use common::{
    syscall::Syscall,
    syscall_output::SyscallOutput,
    syscall_print::{SyscallPrintError, SyscallPrintOutput},
    syscall_start_recording_keyboard::SyscallStartRecordingKeyboardInput,
    syscall_take_frame_buffer::{
        TakeFrameBufferError, TakeFrameBufferOutput, TakeFrameBufferOutputData,
    },
    syscall_uuids::SYSCALL_EXISTS,
};
use uuid::Uuid;
use x86_64::VirtAddr;

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
pub unsafe fn syscall_uuid(uuid: Uuid, inputs: [u64; 5]) -> u64 {
    let (input0, input1) = uuid.as_u64_pair();
    unsafe {
        syscall_internal(
            input0, input1, inputs[0], inputs[1], inputs[2], inputs[3], inputs[4],
        )
    }
}

pub fn syscall_exists(uuid: Uuid) -> bool {
    let (input0, input1) = uuid.as_u64_pair();
    let return_value = unsafe { syscall_uuid(SYSCALL_EXISTS, [input0, input1, 0, 0, 0]) };
    match return_value {
        1 => true,
        0 => false,
        _ => unreachable!(),
    }
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

pub fn syscall_poll_keyboard(buffer: &mut [MaybeUninit<u8>]) -> &mut [u8] {
    let count = syscall(&Syscall::PollKeyboard(buffer.into())) as usize;
    unsafe { buffer[..count].assume_init_mut() }
}

pub fn syscall_allocate_pages(total_pages: u64) -> VirtAddr {
    VirtAddr::new(syscall(&Syscall::AllocatePages(total_pages)))
}

/// Set your handler to `unsafe` to avoid accidentally calling it in your code.
/// Call [`syscall_done_with_interrupt_handler`](syscall_done_with_interrupt_handler) at the end of your handler.
pub type KeyboardInterruptHandler = unsafe extern "sysv64" fn() -> !;

pub fn syscall_set_keyboard_interrupt_handler(handler: Option<KeyboardInterruptHandler>) {
    syscall(&Syscall::SetKeyboardInterruptHandler(
        handler.map(|handler| (handler as *const ()).into()),
    ));
}

pub fn syscall_done_with_interrupt_handler() -> ! {
    syscall(&Syscall::DoneWithInterruptHandler);
    unreachable!()
}

pub fn syscall_disable_and_defer_my_interrupts() {
    syscall(&Syscall::DisableAndDeferMyInterrupts);
}

pub fn syscall_enable_and_catch_up_on_my_interrupts() {
    syscall(&Syscall::EnableAndCatchUpOnMyInterrupts);
}

pub fn syscall_enable_my_interrupts_and_wait_until_one_happens() {
    syscall(&Syscall::EnableMyInterruptsAndWaitUntilOneHappens);
}

pub fn syscall_enable_hpet() {
    syscall(&Syscall::EnableHpet);
}

pub fn syscall_hpet_read_main_counter_value() -> u64 {
    syscall(&Syscall::HpetReadMainCounterValue)
}

pub fn syscall_get_hpet_main_counter_period() -> u32 {
    syscall(&Syscall::GetHpetMainCounterPeriod) as u32
}
