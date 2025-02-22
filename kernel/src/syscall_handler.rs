use core::{arch::naked_asm, cmp::Ordering, mem::MaybeUninit, ops::DerefMut, str};

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
use spin::Mutex;
use x86_64::{
    instructions::interrupts,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTableFlags, Size4KiB,
    },
    PrivilegeLevel, VirtAddr,
};

use crate::{
    context::{AnyContext, Context, SyscallContext},
    cool_keyboard_interrupt_handler::{CoolKeyboard, USER_SPACE_INTERRUPT_HANDLER},
    enter_user_mode::enter_user_mode,
    hlt_loop::hlt_loop,
    memory::BootInfoFrameAllocator,
    modules::syscall::syscall_handler::SyscallHandler,
    user_space_state::State,
};

pub struct UserSpaceMemInfo {
    user_space_heap_start: VirtAddr,
    allocated_pages: u64,
}

impl UserSpaceMemInfo {
    pub fn new(user_space_heap_start: VirtAddr) -> Self {
        Self {
            user_space_heap_start,
            allocated_pages: 0,
        }
    }
}

struct StaticStuff {
    frame_buffer: Option<&'static mut FrameBuffer>,
    mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
    frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
    cool_keyboard: CoolKeyboard,
    user_space_mem_info: Arc<Mutex<Option<UserSpaceMemInfo>>>,
    state: Arc<Mutex<State>>,
}

static STATIC_STUFF: OnceCell<StaticStuff> = OnceCell::uninit();

// save the registers, handle the syscall and return to usermode
#[naked]
unsafe extern "sysv64" fn raw_syscall_handler() {
    unsafe {
        naked_asm!("\
            // backup registers for sysretq
            push rcx
            push r11

            // save callee-saved registers on the stack
            push rbp
            push rbx
            push r12
            push r13
            push r14
            push r15

            // Save rcx = rax because rcx is not used as a syscall input but rax is, and we need to save rax
            mov rcx, rax

            // Save caller-saved registers
            push rdi
            push rsi
            push rdx
            push rcx
            push r8
            push r9
            push r10
            push r11

            // Get the temp rsp, it will be outputted in rax
            call {get_temp_rsp}

            // Restore caller-saved registers
            pop r11
            pop r10
            pop r9
            pop r8
            pop rcx
            pop rdx
            pop rsi
            pop rdi

            // Switch to temp stack
            mov rbp, rsp
            mov rsp, rax

            // Get the rax from user space back (rax = rcx)
            mov rax, rcx

            // Call the function
            // Convert `syscall`s `r10` input to `sysv64`s `rcx` input
            mov rcx, r10
            // After the first 6 inputs, additional inputs go on the stack **in reverse order**. So we put `rax` on the stack
            push rbp // I added an extra input which is the user space stack pointer
            push rax // Move rax to the stack which is where additional inputs go in sysv64
            call {handle_syscall}

            // asm version of unreachable!() un rust
            ud2

            // This is what we would do if we were going to sysretq in this function, but then I changed it so that the handle_syscall function does the sysretq itself
            // Switch back to the old stack
            // mov rsp, rbp

            // // restore callee-saved registers from the stack
            // pop r15
            // pop r14
            // pop r13
            // pop r12
            // pop rbx
            // pop rbp

            // // restore registers from the stack for sysretq
            // pop r11
            // pop rcx

            // // go back to user mode
            // sysretq
            ",
            handle_syscall = sym handle_syscall,
            get_temp_rsp = sym get_temp_rsp
        );
    }
}

const SYSCALL_HANDLER: SyscallHandler =
    unsafe { SyscallHandler::new_unchecked(raw_syscall_handler) };

const TEMP_STACK_SIZE: usize = 0x10000;
#[repr(C, align(16))]
struct TempStack([u8; TEMP_STACK_SIZE]);
static mut TEMP_STACK: MaybeUninit<TempStack> = MaybeUninit::uninit();

extern "sysv64" fn get_temp_rsp() -> u64 {
    let temp_stack_start = VirtAddr::from_ptr(unsafe {
        // Safety: We set the rsp in a way so that the same part of the temp stack isn't used at the same time
        #[allow(static_mut_refs)]
        TEMP_STACK.as_ptr()
    });
    let temp_stack_end = temp_stack_start + TEMP_STACK_SIZE as u64;
    let temp_stack_range = temp_stack_start..temp_stack_end;
    let state = STATIC_STUFF.try_get().unwrap().state.lock();
    let contexts = &state.as_ref().unwrap().stack_of_saved_contexts;
    let temp_stack_rsp = contexts
        .iter()
        .rev()
        .filter_map(|context| match context {
            AnyContext::Full(context) => Some(context),
            AnyContext::Syscall(_) => None,
        })
        .filter(|context| context.privilege_level() == PrivilegeLevel::Ring0)
        .find(|context| temp_stack_range.contains(&VirtAddr::new(context.rsp)))
        .map(|context| {
            // Align to 16 bytes
            VirtAddr::new(context.rsp.div_floor(16) * 16)
            // TODO: Maybe check if we are gonna have a stack overflow (if the new rsp is already below the start of the temp stack)
        })
        .unwrap_or(temp_stack_end);
    // log::info!(
    //     "Contexts: {:#x?}. Temp stack rsp: {:?}. Temp stack range: {:?}",
    //     contexts,
    //     temp_stack_rsp,
    //     temp_stack_range
    // );
    temp_stack_rsp.as_u64()
}

extern "sysv64" fn handle_syscall(
    input0: u64,
    input1: u64,
    input2: u64,
    input3: u64,
    input4: u64,
    input5: u64,
    input6: u64,
    user_space_stack_pointer: u64,
) -> ! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct PushedRegisters {
        pub r15: u64,
        pub r14: u64,
        pub r13: u64,
        pub r12: u64,
        pub rbx: u64,
        pub rbp: u64,
        pub r11: u64,
        pub rcx: u64,
    }
    let rsp_to_restore = user_space_stack_pointer + size_of::<PushedRegisters>() as u64;
    let get_syscall_context = |return_value: u64| {
        let pushed_registers = unsafe { *(user_space_stack_pointer as *const PushedRegisters) };
        let rsp_to_restore = user_space_stack_pointer + size_of::<PushedRegisters>() as u64;
        SyscallContext {
            r15: pushed_registers.r15,
            r14: pushed_registers.r14,
            r13: pushed_registers.r13,
            r12: pushed_registers.r12,
            rbx: pushed_registers.rbx,
            rbp: pushed_registers.rbp,
            r11: pushed_registers.r11,
            rcx: pushed_registers.rcx,
            rax: return_value,
            rsp: rsp_to_restore,
        }
    };
    let inputs = [input0, input1, input2, input3, input4, input5, input6];
    let return_value = match Syscall::deserialize_from_input(inputs) {
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
            Syscall::Exit => {
                // Nothing to do
                hlt_loop();
            }
            Syscall::StartRecordingKeyboard(input) => {
                STATIC_STUFF.try_get().unwrap().cool_keyboard.enable(input);
                Default::default()
            }
            Syscall::PollKeyboard(dest) => {
                let dest_ptr: *mut u8 = dest.into();
                if !dest_ptr.is_null()
                    && dest_ptr.is_aligned()
                    && VirtAddr::from_ptr(dest_ptr.wrapping_add(1))
                        <= VirtAddr::new_truncate(KERNEL_VIRT_MEM_START)
                {
                    match STATIC_STUFF
                        .try_get()
                        .unwrap()
                        .cool_keyboard
                        .queue()
                        .queue()
                    {
                        Some(queue) => {
                            let slice = unsafe { dest.to_slice_mut::<u8>() };
                            let count = {
                                let mut count = 0;
                                while let Some(slot) = slice.get_mut(count) {
                                    match queue.pop() {
                                        Some(scan_code) => {
                                            *slot = scan_code;
                                        }
                                        None => {
                                            break;
                                        }
                                    }
                                    count += 1;
                                }
                                count
                            };
                            count as u64
                        }
                        None => 0,
                    }
                } else {
                    0
                }
            }
            Syscall::AllocatePages(pages) => {
                let stuff = STATIC_STUFF.try_get().unwrap();
                let mut user_space_mem_info = stuff.user_space_mem_info.lock();
                let UserSpaceMemInfo {
                    user_space_heap_start,
                    allocated_pages,
                } = user_space_mem_info.as_mut().unwrap();

                // FIXME: Check for situations where a ton of pages are requested
                match (*allocated_pages).cmp(&pages) {
                    Ordering::Less => {
                        let mut frame_allocator = stuff.frame_allocator.lock();
                        let mut mapper = stuff.mapper.lock();
                        for i in *allocated_pages..pages {
                            unsafe {
                                mapper.map_to(
                                    Page::from_start_address(*user_space_heap_start).unwrap() + i,
                                    // FIXME: Handle errors
                                    frame_allocator.allocate_frame().unwrap(),
                                    PageTableFlags::PRESENT
                                        | PageTableFlags::USER_ACCESSIBLE
                                        | PageTableFlags::WRITABLE,
                                    frame_allocator.deref_mut(),
                                )
                            }
                            .unwrap()
                            .flush();
                        }
                    }
                    Ordering::Equal => {}
                    Ordering::Greater => {
                        // FIXME: Deallocate pages
                    }
                }
                user_space_heap_start.as_u64()
            }
            Syscall::SetKeyboardInterruptHandler(user_space_interrupt) => {
                STATIC_STUFF
                    .try_get()
                    .unwrap()
                    .cool_keyboard
                    .set_user_space_interrupt(
                        user_space_interrupt.map(|syscall_pointer| {
                            VirtAddr::from_ptr::<()>(syscall_pointer.into())
                        }),
                    );
                Default::default()
            }
            Syscall::DoneWithInterruptHandler => {
                // Make sure lock is dropped
                enum Action {
                    JmpToUserMode(VirtAddr, VirtAddr),
                    RestoreContext(AnyContext),
                }
                let action = {
                    {
                        // log::info!(
                        //     "[{syscall:?}] State: {:#x?}",
                        //     STATIC_STUFF.try_get().unwrap().state.lock().deref()
                        // );
                    };
                    let mut user_space_state = STATIC_STUFF.try_get().unwrap().state.lock();
                    let user_space_state = user_space_state.as_mut().unwrap();
                    // TODO: Return with `Err`
                    if !user_space_state.in_keyboard_interrupt_handler {
                        unreachable!("{:?} called outside of interrupt handler", syscall);
                    }
                    user_space_state.in_keyboard_interrupt_handler = false;
                    if !user_space_state.keyboard_interrupt_queued {
                        Action::RestoreContext(
                            user_space_state.stack_of_saved_contexts.pop().unwrap(),
                        )
                    } else {
                        // log::info!("Entering queued keyboard interrupt handler");
                        user_space_state.keyboard_interrupt_queued = false;
                        match USER_SPACE_INTERRUPT_HANDLER.lock().as_ref() {
                            Some(user_space_interrupt_handler) => {
                                let interrupt_handler_stack_end = VirtAddr::new(
                                    match user_space_state.stack_of_saved_contexts.last().unwrap() {
                                        AnyContext::Full(full_context) => full_context.rsp,
                                        AnyContext::Syscall(syscall_context) => syscall_context.rsp,
                                    },
                                );
                                user_space_state.in_keyboard_interrupt_handler = true;
                                Action::JmpToUserMode(
                                    *user_space_interrupt_handler,
                                    interrupt_handler_stack_end,
                                )
                            }
                            None => Action::RestoreContext(
                                user_space_state.stack_of_saved_contexts.pop().unwrap(),
                            ),
                        }
                    }
                };
                match action {
                    Action::RestoreContext(context) => {
                        unsafe { context.context().restore() };
                    }
                    Action::JmpToUserMode(code, stack_end) => {
                        unsafe { enter_user_mode(code, stack_end) };
                    }
                }
            }
            Syscall::DisableAndDeferMyInterrupts => {
                STATIC_STUFF
                    .try_get()
                    .unwrap()
                    .state
                    .lock()
                    .as_mut()
                    .unwrap()
                    .interrupts_enabled = false;
                Default::default()
            }
            Syscall::EnableAndCatchUpOnMyInterrupts => {
                let interrupt_to_jmp_to = {
                    let mut user_space_state = STATIC_STUFF.try_get().unwrap().state.lock();
                    let user_space_state = user_space_state.as_mut().unwrap();
                    user_space_state.interrupts_enabled = true;
                    if user_space_state.keyboard_interrupt_queued {
                        user_space_state.keyboard_interrupt_queued = false;
                        if let Some(user_space_interrupt_handler) =
                            USER_SPACE_INTERRUPT_HANDLER.lock().as_ref()
                        {
                            let interrupt_handler_stack_end = VirtAddr::new(rsp_to_restore);
                            user_space_state.in_keyboard_interrupt_handler = true;
                            user_space_state
                                .stack_of_saved_contexts
                                .push_within_capacity(AnyContext::Syscall(get_syscall_context(
                                    Default::default(),
                                )))
                                .unwrap();
                            Some((*user_space_interrupt_handler, interrupt_handler_stack_end))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some((code, stack_end)) = interrupt_to_jmp_to {
                    unsafe { enter_user_mode(code, stack_end) }
                }
                Default::default()
            }
            Syscall::EnableMyInterruptsAndWaitUntilOneHappens => {
                enum Action {
                    Return(u64),
                    JmpToUserMode(VirtAddr, VirtAddr),
                    WaitForInterruptToHappen,
                }
                let action = {
                    let mut user_space_state = STATIC_STUFF.try_get().unwrap().state.lock();
                    let user_space_state = user_space_state.as_mut().unwrap();
                    user_space_state.interrupts_enabled = true;
                    if user_space_state.keyboard_interrupt_queued {
                        user_space_state.keyboard_interrupt_queued = false;
                        if let Some(user_space_interrupt_handler) =
                            USER_SPACE_INTERRUPT_HANDLER.lock().as_ref()
                        {
                            let interrupt_handler_stack_end = VirtAddr::new(rsp_to_restore);
                            user_space_state.in_keyboard_interrupt_handler = true;
                            user_space_state
                                .stack_of_saved_contexts
                                .push_within_capacity(AnyContext::Syscall(get_syscall_context(
                                    Default::default(),
                                )))
                                .unwrap();
                            Action::JmpToUserMode(
                                *user_space_interrupt_handler,
                                interrupt_handler_stack_end,
                            )
                        } else {
                            // Consider the interrupt to have happened (cuz there is no handler)
                            Action::Return(Default::default())
                        }
                    } else {
                        // Wait until one happens
                        user_space_state
                            .stack_of_saved_contexts
                            .push_within_capacity(AnyContext::Syscall(get_syscall_context(
                                Default::default(),
                            )))
                            .unwrap();
                        Action::WaitForInterruptToHappen
                    }
                };
                match action {
                    Action::JmpToUserMode(code, stack_end) => unsafe {
                        enter_user_mode(code, stack_end)
                    },
                    Action::WaitForInterruptToHappen => {
                        interrupts::enable_and_hlt();
                        unreachable!()
                    }
                    Action::Return(return_value) => return_value,
                }
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
    };
    let syscall_context = get_syscall_context(return_value);
    unsafe { syscall_context.restore() };
}

pub fn get_syscall_handler(
    frame_buffer: Option<&'static mut FrameBuffer>,
    mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
    frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
    cool_keyboard: CoolKeyboard,
    user_space_mem_info: Arc<spin::Mutex<Option<UserSpaceMemInfo>>>,
    state: Arc<Mutex<State>>,
) -> SyscallHandler {
    STATIC_STUFF
        .try_init_once(|| StaticStuff {
            frame_buffer,
            mapper,
            frame_allocator,
            cool_keyboard,
            user_space_mem_info,
            state,
        })
        .unwrap();
    SYSCALL_HANDLER
}
