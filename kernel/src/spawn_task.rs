use core::{mem::MaybeUninit, slice};

use alloc::boxed::Box;
use anyhow::{anyhow, Context};
use common::{mem::KERNEL_VIRT_MEM_START, ram_disk::RamDisk};
use elf::{endian::NativeEndian, ElfBytes};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{FrameAllocator, Mapper, Page, PageSize, PageTableFlags, Size4KiB},
    VirtAddr,
};

use crate::{
    iopb_size::IOPB_SIZE,
    tasks::{ReadyToStartState, StackChunk, Task, TaskState, TaskType, UserTaskData, TASKS},
};

/// Only specifies `WRITABLE` and `NO_EXECUTE` if needed. Other flags such as `PRESENT` and `USER_ACCESSIBLE` must be added.
pub fn elf_flags_to_page_table_flags(elf_flags: u32) -> PageTableFlags {
    let mut page_table_flags = PageTableFlags::empty();
    if elf_flags & 0b001 == 0 {
        page_table_flags |= PageTableFlags::NO_EXECUTE;
    }
    if elf_flags & 0b010 != 0 {
        page_table_flags |= PageTableFlags::WRITABLE;
    }
    page_table_flags
}

pub fn spawn_task(
    program: RamDisk<'static>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    mapper: &mut impl Mapper<Size4KiB>,
) -> anyhow::Result<()> {
    let elf = ElfBytes::<NativeEndian>::minimal_parse(&program.elf)?;
    let loadable_segments = elf
        .segments()
        .ok_or(anyhow!("No segments"))?
        .into_iter()
        .filter(|segment| segment.p_type == 1);
    let start_symbol = {
        let (symbols_parsing_table, symbols_strings) = elf
            .symbol_table()?
            .ok_or(anyhow!("No symbols / symbol strings"))?;
        symbols_parsing_table
            .into_iter()
            .filter(|symbol| !symbol.is_undefined())
            .find_map(
                |symbol| match symbols_strings.get(symbol.st_name as usize) {
                    Ok(symbol_string) => match symbol_string {
                        "_start" => Some(Ok(symbol)),
                        _ => None,
                    },
                    Err(e) => Some(Err(e)),
                },
            )
            .ok_or(anyhow!("_start not found"))?
            .context("Error finding _start symbol")?
    };
    let mut elf_end = Page::<Size4KiB>::from_start_address(VirtAddr::zero()).unwrap();
    for segment in loadable_segments {
        let segment_data = elf.segment_data(&segment)?;
        // log::info!("Must map segment accessible to the kernel at {:p} to virtual address 0x{:x} with size 0x{:x} and copy 0x{:x} bytes, with alignment down 0x{:x} with flags 0b{:b}", segment_data, segment.p_vaddr, segment.p_memsz, segment.p_filesz, segment.p_align, segment.p_flags);
        let page_range = {
            let start = Page::<Size4KiB>::from_start_address(
                VirtAddr::new(segment.p_vaddr).align_down(Size4KiB::SIZE),
            )
            .unwrap();
            let end = Page::from_start_address(
                (VirtAddr::new(segment.p_vaddr) + segment.p_memsz).align_up(Size4KiB::SIZE),
            )
            .unwrap();
            start..end
        };

        fn set_phys_frame(
            frame_allocator: &mut impl FrameAllocator<Size4KiB>,
            mapper: &mut impl Mapper<Size4KiB>,
            page: Page,
            f: impl FnOnce(&mut [u8]),
            final_flags: PageTableFlags,
        ) -> anyhow::Result<()> {
            let phys_frame = frame_allocator
                .allocate_frame()
                .ok_or(anyhow!("Failed to allocate frame"))?;
            if page.start_address().is_null() {
                let temp_page = Page::from_start_address(VirtAddr::new_truncate(
                    KERNEL_VIRT_MEM_START - Size4KiB::SIZE,
                ))
                .unwrap();
                unsafe {
                    mapper.map_to(
                        temp_page,
                        phys_frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            // FIXME: Remove user accessible. But for now, we keep it cuz of [a bug in `update_flags`](https://github.com/rust-osdev/x86_64/issues/534)
                            | PageTableFlags::USER_ACCESSIBLE,
                        frame_allocator,
                    )
                }
                .map_err(|_| anyhow!("Failed to map page"))?
                .flush();

                let slice = unsafe {
                    slice::from_raw_parts_mut::<u8>(
                        temp_page.start_address().as_mut_ptr(),
                        temp_page.size() as usize,
                    )
                };
                f(slice);

                mapper
                    .unmap(temp_page)
                    .map_err(|_| anyhow!("Error un-mapping temp page"))?
                    .1
                    .flush();
                unsafe { mapper.map_to(page, phys_frame, final_flags, frame_allocator) }
                    .map_err(|_| anyhow!("Error mapping page"))?
                    .flush();
            } else {
                unsafe {
                    mapper.map_to(
                        page,
                        phys_frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            // FIXME: Remove user accessible. But for now, we keep it cuz of [a bug in `update_flags`](https://github.com/rust-osdev/x86_64/issues/534)
                            | PageTableFlags::USER_ACCESSIBLE,
                        frame_allocator,
                    )
                }
                .map_err(|_| anyhow!("Failed to map page"))?
                .flush();

                let slice = unsafe {
                    slice::from_raw_parts_mut::<u8>(
                        page.start_address().as_mut_ptr(),
                        page.size() as usize,
                    )
                };

                f(slice);

                unsafe { mapper.update_flags(page, final_flags) }
                    .map_err(|_| anyhow!("Failed to update flags"))?
                    .flush();
            }
            Ok(())
        }

        for (page_index, page) in page_range.clone().enumerate() {
            set_phys_frame(
                frame_allocator,
                mapper,
                page,
                |slice| {
                    // Zero the phys frame to be secure
                    slice.fill(Default::default());
                    // Copy the data
                    let dest_start = if page_index == 0 {
                        segment.p_vaddr % segment.p_align
                    } else {
                        0
                    };
                    let already_copied = match page_index {
                        0 => 0,
                        n => Size4KiB::SIZE * n as u64 - (segment.p_vaddr % segment.p_align),
                    };
                    let dest_end =
                        (dest_start + (segment.p_filesz - already_copied)).min(slice.len() as u64);

                    let src_start = already_copied;
                    let src_end = src_start + (dest_end - dest_start);
                    // log::warn!(
                    //     "Page index: {}, copy bytes: {}, already copied: {}, Copying to frame: {:?} from segment data: {:?}",
                    //     page_index,
                    //     segment.p_filesz,
                    //     already_copied,
                    //     dest_start..dest_end,
                    //     src_start..src_end,
                    // );
                    slice[dest_start as usize..dest_end as usize]
                        .copy_from_slice(&segment_data[src_start as usize..src_end as usize]);
                },
                PageTableFlags::PRESENT
                    | PageTableFlags::USER_ACCESSIBLE
                    | elf_flags_to_page_table_flags(segment.p_flags),
            )?;
        }

        elf_end = elf_end.max(page_range.end);
    }

    // Do relocations
    // Warning: I don't fully understand this and the implementation may only work under certain assumptions
    if let Some(section_headers) = elf.section_headers() {
        for section_header in section_headers {
            if section_header.sh_type == 4 {
                let relas = elf.section_data_as_relas(&section_header)?;
                for rela in relas {
                    match rela.r_type {
                        8 => {
                            // TODO: The offset needs to be added to the base virtual address. The base virtual address may not be 0.
                            let virt_addr = VirtAddr::new(rela.r_offset);
                            let mem_to_replace = virt_addr.as_mut_ptr::<u64>();
                            unsafe { *mem_to_replace = rela.r_addend as u64 };
                        }
                        _ => log::warn!("Not applying rela: {:?}", rela),
                    }
                }
            }
        }
    }

    const USER_SPACE_STACK_SIZE: usize = 0x3000;
    let page_count = (USER_SPACE_STACK_SIZE as u64).div_ceil(Size4KiB::SIZE);
    let stack_start = elf_end;
    let stack_end = stack_start + page_count;
    let stack_pages = stack_start..stack_end;
    log::info!("User space Stack: {stack_pages:?}");
    for page in stack_pages {
        unsafe {
            mapper.map_to(
                page,
                frame_allocator
                    .allocate_frame()
                    .ok_or(anyhow!("Failed to allocate frame for stack"))?,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                frame_allocator,
            )
        }
        .map_err(|_| anyhow!("Failed to map page"))?
        .flush();

        let slice = unsafe {
            slice::from_raw_parts_mut::<u8>(page.start_address().as_mut_ptr(), page.size() as usize)
        };
        // Zero the stack to avoid exposing data
        slice.fill(Default::default());

        // Now that the page is zeroed, make it user accessible
        unsafe {
            mapper.update_flags(
                page,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::NO_EXECUTE,
            )
        }
        .map_err(|_| anyhow!("Failed to update stack page flags"))?
        .flush();
    }

    let start_addr = VirtAddr::new(start_symbol.st_value);

    let task = Task {
        task_type: TaskType::User(UserTaskData {
            cr3: Cr3::read().0,
            kernel_stack: Box::new({
                // This is what Linux uses so let's do it too: https://lwn.net/Articles/600649/
                const SYSCALL_STACK_SIZE: u64 = Size4KiB::SIZE * 4;
                const CHUNKS: usize = SYSCALL_STACK_SIZE as usize / size_of::<StackChunk>();
                MaybeUninit::uninit_array::<CHUNKS>()
            }),
            iopb: {
                let mut iopb = [u8::MAX; IOPB_SIZE];
                program.permissions.ports.iter().for_each(|allowed_port| {
                    iopb[allowed_port.div_floor(8) as usize] &= !(1 << (allowed_port % 8));
                });
                iopb
            },
        }),
        state: TaskState::ReadyToStart(ReadyToStartState {
            instruction_pointer: start_addr,
            stack_pointer: stack_end.start_address(),
        }),
    };
    TASKS.lock().push(task);

    Ok(())
}
