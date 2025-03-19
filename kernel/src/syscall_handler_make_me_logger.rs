use core::ptr::NonNull;

use common::{syscall_pointer::SyscallPointer, syscall_uuids::SyscallMakeMeLoggerOutput};
use uuid::Uuid;
use x86_64::VirtAddr;
use zerocopy::TryFromBytes;

use crate::{
    modules::syscall::syscall_handler_closure::PushedRegisters,
    syscall_handler_closure::Includes,
    tasks::{get_running_task, TASKS},
};

pub fn get_syscall_make_me_logger_handler(
) -> impl Fn([u64; 5], &mut PushedRegisters, &dyn Includes<Uuid>) -> ! + Send + Sync {
    |input, pushed_registers, _| {
        let input =
            postcard::from_bytes::<SyscallPointer>(bytemuck::try_cast_slice(&input[..]).unwrap())
                .expect("Bad syscall. Terminate process.");
        let mut output = NonNull::new(<*mut [u8; size_of::<SyscallMakeMeLoggerOutput>()]>::from(
            input,
        ))
        .expect("terminate");
        // FIXME: Check that address is accessible by user space and will not cause a page fault (like if the page is not mapped)
        let output = SyscallMakeMeLoggerOutput::try_mut_from_bytes(unsafe { output.as_mut() })
            .expect("terminate");

        let mut tasks = TASKS.lock();
        let user_space_stream_address = VirtAddr::try_new(output.address).expect("terminate");

        todo!()
    }
}
