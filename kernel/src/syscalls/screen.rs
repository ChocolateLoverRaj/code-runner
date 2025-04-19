use common::{
    screen_info::ScreenInfoWithAddress,
    syscall_uuids::{
        Syscall, SyscallReleaseScreen, SyscallReleaseScreenError, SyscallTakeScreen,
        SyscallTakeScreenError,
    },
};
use limine::{framebuffer::Framebuffer, response::FramebufferResponse};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{Mapper, PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr,
};

use crate::{
    cpu_local_data::get_local,
    find_contiguous_unused_virtual_memory::find_contiguous_unused_virtual_memory,
    get_offset_page_table::get_offset_page_table,
    hhdm_offset::HhdmOffset,
    logger_3,
    page_tables_recursive_iterator::PageTablesRecursiveIterator,
    physical_memory::{PhysicalMemoryFrameAllocator, UsedBy, PHYSICAL_MEMORY},
    screen_lock::{TaskUsingScreen, WhoIsUsingScreen, SCREEN_LOCK},
    tasks::TASKS,
    terminate_current_task::terminate_current_task,
};

use super::syscall_handlers::SyscallHandler2;

pub struct SyscallTakeScreenHandler {
    hhdm_offset: HhdmOffset,
    frame_buffer: Option<Framebuffer<'static>>,
}
impl SyscallTakeScreenHandler {
    pub fn new(
        hhdm_offset: HhdmOffset,
        frame_buffer: Option<&'static FramebufferResponse>,
    ) -> Self {
        Self {
            hhdm_offset,
            frame_buffer: frame_buffer.and_then(|f| f.framebuffers().next()),
        }
    }
}
impl SyscallHandler2 for SyscallTakeScreenHandler {
    type Syscall = SyscallTakeScreen;

    fn handle_syscall(
        &self,
        _input: <Self::Syscall as common::syscall_uuids::Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as common::syscall_uuids::Syscall>::Output {
        enum Action {
            Return(<SyscallTakeScreen as Syscall>::Output),
            Terminate,
        }
        let action = {
            let task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
            let tasks = TASKS.lock();
            let task = tasks.get(&task_id).unwrap();
            if task.permissions.screen {
                if let Some(frame_buffer) = &self.frame_buffer {
                    let start =
                        PhysAddr::new(frame_buffer.addr() as u64 - u64::from(self.hhdm_offset));
                    let byte_len = frame_buffer.pitch() * frame_buffer.height();
                    if start.is_aligned(Size4KiB::SIZE) && byte_len.is_multiple_of(Size4KiB::SIZE) {
                        let mut screen_lock = SCREEN_LOCK.lock();
                        let setup_frame_buffer = |screen_lock: &mut Option<WhoIsUsingScreen>| {
                            let page_count = byte_len / 0x1000;
                            let mut o = get_offset_page_table(self.hhdm_offset);
                            let page_range = find_contiguous_unused_virtual_memory(
                                unsafe {
                                    PageTablesRecursiveIterator::new(
                                        self.hhdm_offset,
                                        Cr3::read().0,
                                        0,
                                    )
                                },
                                page_count,
                            )
                            .unwrap();
                            *screen_lock = Some(WhoIsUsingScreen::Task(TaskUsingScreen {
                                id: task_id,
                                mapped_start: page_range.start,
                            }));
                            let starting_frame =
                                PhysFrame::<Size4KiB>::from_start_address(start).unwrap();
                            let mut physical_memory = PHYSICAL_MEMORY.try_get().unwrap().lock();
                            let mut frame_allocator = PhysicalMemoryFrameAllocator::new(
                                &mut physical_memory,
                                UsedBy::UserSpace(task_id),
                            );
                            for (i, page) in page_range.clone().enumerate() {
                                unsafe {
                                    o.map_to(
                                        page,
                                        starting_frame + i as u64,
                                        PageTableFlags::PRESENT
                                            | PageTableFlags::WRITABLE
                                            | PageTableFlags::USER_ACCESSIBLE
                                            | PageTableFlags::NO_EXECUTE
                                            | PageTableFlags::WRITE_THROUGH,
                                        &mut frame_allocator,
                                    )
                                    .unwrap()
                                    .flush();
                                }
                            }
                            Action::Return(Ok(unsafe {
                                ScreenInfoWithAddress::new(
                                    page_range.start.start_address().as_u64() as usize,
                                    frame_buffer.into(),
                                )
                            }))
                        };
                        match &*screen_lock {
                            Some(WhoIsUsingScreen::KernelLogger) => {
                                logger_3::stop_using_frame_buffer();
                                setup_frame_buffer(&mut screen_lock)
                            }
                            Some(WhoIsUsingScreen::Task(_)) => {
                                Action::Return(Err(SyscallTakeScreenError::InUse))
                            }
                            None => setup_frame_buffer(&mut screen_lock),
                        }
                    } else {
                        Action::Return(Err(SyscallTakeScreenError::WouldNotBeSecure))
                    }
                } else {
                    Action::Return(Err(SyscallTakeScreenError::NoScreenAvailable))
                }
            } else {
                log::warn!(
                    "Task {:?} tried to take screen when it doesn't have permission. Terminating.",
                    task_id
                );
                Action::Terminate
            }
        };
        match action {
            Action::Return(r) => r,
            Action::Terminate => terminate_current_task(),
        }
    }
}

pub struct SyscallReleaseScreenHandler {
    hhdm_offset: HhdmOffset,
    frame_buffer: Option<Framebuffer<'static>>,
}
impl SyscallReleaseScreenHandler {
    pub fn new(
        hhdm_offset: HhdmOffset,
        frame_buffer: Option<&'static FramebufferResponse>,
    ) -> Self {
        Self {
            hhdm_offset,
            frame_buffer: frame_buffer.and_then(|f| f.framebuffers().next()),
        }
    }
}
impl SyscallHandler2 for SyscallReleaseScreenHandler {
    type Syscall = SyscallReleaseScreen;

    fn handle_syscall(
        &self,
        _input: <Self::Syscall as Syscall>::Input,
        _pushed_registers: &mut super::raw_syscall_handler::PushedRegisters,
        _syscalls: &dyn super::syscall_handlers::Includes<uuid::Uuid>,
    ) -> <Self::Syscall as Syscall>::Output {
        log::debug!("{} called", core::any::type_name::<SyscallReleaseScreen>());
        let task_id = get_local().unwrap().task_data.lock().current_task.unwrap();
        let mut screen_lock = SCREEN_LOCK.lock();
        if let Some(WhoIsUsingScreen::Task(task_using_screen)) = &*screen_lock {
            if task_using_screen.id == task_id {
                let mut o = get_offset_page_table(self.hhdm_offset);
                let frame_buffer = self.frame_buffer.as_ref().unwrap();
                let frame_buffer_bytes = frame_buffer.pitch() * frame_buffer.height();
                let page_count = frame_buffer_bytes / Size4KiB::SIZE;
                for i in 0..page_count {
                    o.unmap(task_using_screen.mapped_start + i)
                        .unwrap()
                        .1
                        .flush();
                }
                *screen_lock = None;
                Ok(())
            } else {
                Err(SyscallReleaseScreenError::NotOwned)
            }
        } else {
            Err(SyscallReleaseScreenError::NotOwned)
        }
    }
}
