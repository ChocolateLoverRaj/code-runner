use core::slice;

use common::ram_disk::RamDisk;
use elf::{endian::NativeEndian, ElfBytes};
use thiserror::Error;
use util::aligned_chunks::AlignedChunks;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageSize, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use crate::{
    boxed_stack::BoxedStack,
    config::SYSCALL_HANDLER_STACK_SIZE,
    get_offset_page_table::get_offset_page_table_with_new_l4,
    hhdm_offset::HhdmOffset,
    physical_memory::{PhysicalMemoryFrameAllocator, UsedBy, PHYSICAL_MEMORY},
    tasks::{ReadyToStartState, Task, TaskId, TaskState, TASKS, TASK_QUEUE},
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
    let elf = ElfBytes::<NativeEndian>::minimal_parse(&program.elf)
        .map_err(SpawnTaskError::ElfParseError)?;
    let loadable_segments = elf
        .segments()
        .ok_or(SpawnTaskError::NoSegments)?
        .into_iter()
        .filter(|segment| segment.p_type == 1);
    let start_symbol = {
        let (symbols_parsing_table, symbols_strings) = elf
            .symbol_table()
            .map_err(SpawnTaskError::ElfParseError)?
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
            .map_err(SpawnTaskError::ElfParseError)?
    };
    let mut elf_end = Page::<Size4KiB>::from_start_address(VirtAddr::zero()).unwrap();
    let mut physical_memory_lock = PHYSICAL_MEMORY.try_get().unwrap().lock();
    let task_id = TaskId::new_unique();
    let mut frame_allocator =
        PhysicalMemoryFrameAllocator::new(&mut physical_memory_lock, UsedBy::UserSpace(task_id));
    // Because the task will have a different Cr3 value, we create a new L4 page table
    let l4 = frame_allocator
        .allocate_frame()
        .ok_or(SpawnTaskError::OutOfPhysMem)?;
    let mut offset_page_table = get_offset_page_table_with_new_l4(l4, hhdm_offset);
    for segment in loadable_segments {
        let segment_data = elf
            .segment_data(&segment)
            .map_err(SpawnTaskError::ElfParseError)?;

        let virtual_range =
            segment.p_vaddr as usize..segment.p_vaddr as usize + segment.p_memsz as usize;
        log::info!("Segment: {:?}. Virtual Range: {:?}", segment, virtual_range);
        for virtual_chunk in virtual_range.clone().aligned_chunks(0x1000) {
            let page =
                Page::<Size4KiB>::containing_address(VirtAddr::new(virtual_chunk.start as u64));
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
            .map_err(SpawnTaskError::MapPageError)?
            // No need to flush because it's not active yet
            .ignore();
            let offset_mapped_addr =
                (phys_frame.start_address().as_u64() + u64::from(hhdm_offset)) as *mut u8;
            let slice = unsafe { core::slice::from_raw_parts_mut(offset_mapped_addr, 0x1000) };
            // Unused memory before the used memory must be zeroed for security
            slice[0..virtual_chunk.start % 0x1000].fill(Default::default());
            let start_index = virtual_chunk.start - virtual_range.start;
            let end_index = virtual_chunk.end - virtual_range.start;
            let copy_len = end_index
                .min(segment.p_filesz as usize)
                .checked_sub(start_index);
            // This is the part that we are actually copying from the ELF file
            if let Some(copy_len) = copy_len {
                slice[virtual_chunk.start % 0x1000..virtual_chunk.start % 0x1000 + copy_len]
                    .copy_from_slice(&segment_data[start_index..start_index + copy_len]);
            }
            // All extra memz that's not part of filez must be zeroed, according to the ELF spec
            slice[virtual_chunk.start % 0x1000 + copy_len.unwrap_or_default()
                ..virtual_chunk.start % 0x1000 + virtual_chunk.len()]
                .fill(Default::default());
            // Unused memory must be zeroed for security
            slice[virtual_chunk.start % 0x1000 + virtual_chunk.len()..].fill(0);

            elf_end = elf_end.max(page + 1);
        }
    }

    // Do relocations
    // Warning: I don't fully understand this and the implementation may only work under certain assumptions
    // FIXME: This will page fault since it's accessing lower half virtual addresses while the current Cr3 does not have those mapped
    if let Some(section_headers) = elf.section_headers() {
        for section_header in section_headers {
            if section_header.sh_type == 4 {
                todo!("Relocation is needed");
                // let relas = elf
                //     .section_data_as_relas(&section_header)
                //     .map_err(|e| SpawnTaskError::ElfParseError(e))?;
                // for rela in relas {
                //     match rela.r_type {
                //         8 => {
                //             // TODO: The offset needs to be added to the base virtual address. The base virtual address may not be 0.
                //             let virt_addr = VirtAddr::new(rela.r_offset);
                //             let mem_to_replace = virt_addr.as_mut_ptr::<u64>();
                //             unsafe { *mem_to_replace = rela.r_addend as u64 };
                //         }
                //         _ => log::warn!("Not applying rela: {:?}", rela),
                //     }
                // }
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
        .map_err(SpawnTaskError::MapPageError)?
        .ignore();
    }

    let start_addr = VirtAddr::new(start_symbol.st_value);

    TASKS.lock().insert(
        task_id,
        Task {
            id: task_id,
            cr3: l4,
            kernel_stack: BoxedStack::new_uninit(SYSCALL_HANDLER_STACK_SIZE),
            permissions: program.meta_data.permissions.clone(),
            state: TaskState::ReadyToStart(ReadyToStartState {
                instruction_pointer: start_addr,
                stack_pointer: stack_end.start_address(),
            }),
        },
    );
    TASK_QUEUE.lock().push(task_id);

    Ok(())
}
