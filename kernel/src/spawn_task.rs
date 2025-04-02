use core::{slice, sync::atomic::Ordering};

use alloc::{boxed::Box, vec::Vec};
use common::ram_disk::RamDisk;
use elf::{endian::NativeEndian, ElfBytes};
use thiserror::Error;
use util::continuous_bool_vec::ContinuousBoolVec;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageSize, PageTableFlags, PhysFrame,
        Size4KiB,
    },
    VirtAddr,
};

use crate::{
    hhdm_offset::HhdmOffset,
    iopb_size::IOPB_SIZE,
    pt_allocator_2::{
        get_offset_page_table::get_offset_page_table_with_new_l4,
        pt_frame_allocator_3::PtFrameAllocator3,
    },
    tasks::{
        ReadyToStartState, StackChunk, Task, TaskState, TaskType, UserTaskData, NEXT_TASK_ID, TASKS,
    },
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

#[derive(Debug, Error)]
pub enum SpawnTaskError {
    #[error("Error parsing ELF")]
    ElfParseError(elf::ParseError),
    #[error("The ELF has no segments")]
    NoSegments,
    #[error("The ELF has no symbol table")]
    NoSymbolTable,
    #[error("The ELF has no start symbol")]
    NoStartSymbol,
    #[error("Ran out of physical memory")]
    OutOfPhysMem,
    #[error("Error mapping page to physical frame")]
    MapPageError(MapToError<Size4KiB>),
}

pub fn spawn_task(
    program: &RamDisk<'static>,
    hhdm_offset: HhdmOffset,
) -> Result<(), SpawnTaskError> {
    let mut task_phys_mem = ContinuousBoolVec::<Vec<_>>::new(usize::MAX, false);

    let elf = ElfBytes::<NativeEndian>::minimal_parse(&program.elf)
        .map_err(|e| SpawnTaskError::ElfParseError(e))?;
    let loadable_segments = elf
        .segments()
        .ok_or(SpawnTaskError::NoSegments)?
        .into_iter()
        .filter(|segment| segment.p_type == 1);
    let start_symbol = {
        let (symbols_parsing_table, symbols_strings) = elf
            .symbol_table()
            .map_err(|e| SpawnTaskError::ElfParseError(e))?
            .ok_or(SpawnTaskError::NoSymbolTable)?;
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
            .ok_or(SpawnTaskError::NoStartSymbol)?
            .map_err(|e| SpawnTaskError::ElfParseError(e))?
    };
    let mut elf_end = Page::<Size4KiB>::from_start_address(VirtAddr::zero()).unwrap();
    let mut frame_allocator = PtFrameAllocator3 {
        f: |frame: PhysFrame<Size4KiB>| {
            let vec_ptr = task_phys_mem.len_vec.as_ptr_range();
            log::warn!("{:?}", vec_ptr);
            task_phys_mem.set(
                {
                    let start = frame.start_address().as_u64() as usize;
                    start..start + 0x1000
                },
                true,
            );
        },
    };
    // Because the task will have a different Cr3 value, we create a new L4 page table
    let l4 = frame_allocator
        .allocate_frame()
        .ok_or(SpawnTaskError::OutOfPhysMem)?;
    let mut offset_page_table = get_offset_page_table_with_new_l4(l4, hhdm_offset);
    for segment in loadable_segments {
        let segment_data = elf
            .segment_data(&segment)
            .map_err(|e| SpawnTaskError::ElfParseError(e))?;
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

        for (page_index, page) in page_range.clone().enumerate() {
            // Map the page
            let phys_frame = frame_allocator
                .allocate_frame()
                .ok_or(SpawnTaskError::OutOfPhysMem)?;
            unsafe {
                offset_page_table.map_to(
                    page,
                    phys_frame,
                    PageTableFlags::PRESENT
                        | PageTableFlags::USER_ACCESSIBLE
                        | elf_flags_to_page_table_flags(segment.p_flags),
                    &mut frame_allocator,
                )
            }
            .map_err(|e| SpawnTaskError::MapPageError(e))?
            // No need to flush because it's not active yet
            .ignore();
            let offset_mapped_addr =
                (phys_frame.start_address().as_u64() + u64::from(hhdm_offset)) as *mut u8;
            let slice = unsafe { core::slice::from_raw_parts_mut(offset_mapped_addr, 0x1000) };
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
        }

        elf_end = elf_end.max(page_range.end);
    }

    // Do relocations
    // Warning: I don't fully understand this and the implementation may only work under certain assumptions
    // FIXME: This will page fault since it's accessing lower half virtual addresses while the current Cr3 does not have those mapped
    if let Some(section_headers) = elf.section_headers() {
        for section_header in section_headers {
            if section_header.sh_type == 4 {
                log::error!("Doing relocation. Will page fault.");
                let relas = elf
                    .section_data_as_relas(&section_header)
                    .map_err(|e| SpawnTaskError::ElfParseError(e))?;
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

    // User space processes must claim entire phys frames. They cannot claim partial frames like the kernel can because then they can still access memory that they don't own.
    let page_count = program.meta_data.stack_size.div_ceil(Size4KiB::SIZE);
    let stack_start = elf_end;
    let stack_end = stack_start + page_count;
    let stack_pages = stack_start..stack_end;
    log::info!("User space Stack: {stack_pages:?}");
    for page in stack_pages {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(SpawnTaskError::OutOfPhysMem)?;

        let slice = unsafe {
            slice::from_raw_parts_mut::<u8>(
                (frame.start_address().as_u64() + u64::from(hhdm_offset)) as *mut u8,
                page.size() as usize,
            )
        };
        // Zero the stack to avoid exposing data
        slice.fill(Default::default());

        unsafe {
            offset_page_table.map_to(
                page,
                frame,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::WRITABLE,
                &mut frame_allocator,
            )
        }
        .map_err(|e| SpawnTaskError::MapPageError(e))?
        .ignore();
    }

    let start_addr = VirtAddr::new(start_symbol.st_value);

    let task = Task {
        id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
        task_type: TaskType::User(UserTaskData {
            cr3: l4,
            kernel_stack: Box::new_uninit_slice(0x200),
            iopb: {
                let mut iopb = [u8::MAX; IOPB_SIZE];
                program
                    .meta_data
                    .permissions
                    .ports
                    .iter()
                    .for_each(|allowed_port| {
                        iopb[allowed_port.div_floor(8) as usize] &= !(1 << (allowed_port % 8));
                    });
                iopb
            },
            log_stream: None,
        }),
        state: TaskState::ReadyToStart(ReadyToStartState {
            instruction_pointer: start_addr,
            stack_pointer: stack_end.start_address(),
        }),
        owned_phys_mem: task_phys_mem,
    };
    TASKS.try_get().unwrap().lock().tasks.push(task);

    Ok(())
}
