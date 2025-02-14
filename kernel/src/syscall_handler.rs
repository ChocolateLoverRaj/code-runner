use core::{ops::DerefMut, str};

use alloc::sync::Arc;
use bootloader_api::info::FrameBuffer;
use common::{
    mem::{KERNEL_VIRT_MEM_START, USER_SPACE_MMIO_START},
    syscall::Syscall,
    syscall_output::SyscallOutput,
    syscall_print::{SyscallPrintError, SyscallPrintOutput},
    syscall_take_frame_buffer::{
        TakeFrameBufferError, TakeFrameBufferOutput, TakeFrameBufferOutputData,
    },
};
use conquer_once::noblock::OnceCell;
use x86_64::{
    structures::paging::{Mapper, OffsetPageTable, Page, PageSize, PageTableFlags, Size4KiB},
    VirtAddr,
};

use crate::{memory::BootInfoFrameAllocator, modules::syscall::handle_syscall::RustSyscallHandler};

struct StaticStuff {
    frame_buffer: Option<&'static mut FrameBuffer>,
    mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
    frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
}

static STATIC_STUFF: OnceCell<StaticStuff> = OnceCell::uninit();

extern "sysv64" fn syscall_handler(
    input0: u64,
    input1: u64,
    input2: u64,
    input3: u64,
    input4: u64,
    input5: u64,
    input6: u64,
) -> u64 {
    let inputs = [input0, input1, input2, input3, input4, input5, input6];
    match Syscall::deserialize_from_input(inputs) {
        Ok(syscall) => match syscall {
            Syscall::Print(message) => {
                let output = SyscallPrintOutput({
                    let pointer: *const u8 = message.into();
                    if pointer.is_null() {
                        Err(SyscallPrintError::PointerIsNull)
                    } else if !pointer.is_aligned() {
                        Err(SyscallPrintError::PointerNotAligned)
                    } else if VirtAddr::from_ptr(pointer.wrapping_add(message.len() as usize))
                        > VirtAddr::new_truncate(KERNEL_VIRT_MEM_START)
                    {
                        Err(SyscallPrintError::PointerNotAllowed)
                    } else {
                        match str::from_utf8(unsafe { message.to_slice() }) {
                            Ok(message) => {
                                log::info!("[U] {:?}", message);
                                Ok(())
                            }
                            Err(_e) => Err(SyscallPrintError::InvalidString),
                        }
                    }
                });
                output.to_syscall_output().unwrap()
            }
            Syscall::TakeFrameBuffer(output) => {
                let return_value = TakeFrameBufferOutput({
                    let output: *mut TakeFrameBufferOutputData = output.into();
                    if output.is_null() {
                        Err(TakeFrameBufferError::PointerIsNull)
                    } else if !output.is_aligned() {
                        Err(TakeFrameBufferError::PointerNotAligned)
                    }
                    //Check if owned by user space
                    else if VirtAddr::from_ptr(output.wrapping_add(1))
                        > VirtAddr::new_truncate(KERNEL_VIRT_MEM_START)
                    {
                        Err(TakeFrameBufferError::PointerNotAllowed)
                    } else {
                        let static_stuff = STATIC_STUFF.try_get().unwrap();
                        match &static_stuff.frame_buffer {
                            Some(frame_buffer) => {
                                if frame_buffer
                                    .buffer()
                                    .as_ptr()
                                    .is_aligned_to(Size4KiB::SIZE as usize)
                                    && frame_buffer
                                        .info()
                                        .byte_len
                                        .is_multiple_of(Size4KiB::SIZE as usize)
                                {
                                    log::warn!("Need to give frame buffer to user space");
                                    let page_count = (frame_buffer.buffer().len() as u64)
                                        .div_ceil(Size4KiB::SIZE);
                                    log::warn!("Will map {} pages", page_count);
                                    let mut mapper = static_stuff.mapper.lock();
                                    let mut frame_allocator = static_stuff.frame_allocator.lock();
                                    log::info!("Got lock...");
                                    let frame_buffer_start_address_in_user_space =
                                        VirtAddr::new_truncate(USER_SPACE_MMIO_START);
                                    let start_page_in_user_space: Page =
                                        Page::<Size4KiB>::from_start_address(
                                            frame_buffer_start_address_in_user_space,
                                        )
                                        .unwrap();
                                    let phys_start = mapper
                                        .translate_page(
                                            Page::from_start_address(VirtAddr::from_ptr(
                                                frame_buffer.buffer().as_ptr(),
                                            ))
                                            .unwrap(),
                                        )
                                        .unwrap();
                                    log::info!("Mapping pages...");
                                    for i in 0..page_count {
                                        unsafe {
                                            mapper
                                                .map_to(
                                                    start_page_in_user_space + i,
                                                    phys_start + i,
                                                    PageTableFlags::PRESENT
                                                        | PageTableFlags::USER_ACCESSIBLE
                                                        | PageTableFlags::WRITABLE
                                                        | PageTableFlags::NO_EXECUTE,
                                                    frame_allocator.deref_mut(),
                                                )
                                                .unwrap()
                                                .flush();
                                        };
                                    }
                                    unsafe {
                                        output.write(TakeFrameBufferOutputData::new(
                                            frame_buffer_start_address_in_user_space.as_u64(),
                                            frame_buffer.info(),
                                        ))
                                    };
                                    Ok(())
                                } else {
                                    log::warn!("Can't give frame buffer to user space because it doesn't have a phys frames to itself.");
                                    Err(TakeFrameBufferError::CannotSecurelyGiveAccess)
                                }
                            }
                            None => {
                                log::warn!("no frame buffer");
                                Err(TakeFrameBufferError::NoFrameBuffer)
                            }
                        }
                    }
                });
                // postcard should never panic
                return_value.to_syscall_output().unwrap()
            }
        },
        Err(e) => {
            log::warn!(
                "Failed to parse syscall inputs (displayed in hex) {:x?}: {:?}",
                inputs,
                e
            );
            // TODO: Stop the user space process
            Default::default()
        }
    }
}

pub fn get_syscall_handler(
    frame_buffer: Option<&'static mut FrameBuffer>,
    mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
    frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
) -> RustSyscallHandler {
    STATIC_STUFF
        .try_init_once(|| StaticStuff {
            frame_buffer,
            mapper,
            frame_allocator,
        })
        .unwrap();
    syscall_handler
}
