use core::{ops::DerefMut, slice};

use alloc::{sync::Arc, vec::Vec};
use anyhow::{anyhow, Context};
use elf::{endian::NativeEndian, ElfBytes};
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use crate::{
    enter_user_mode::enter_user_mode,
    memory::BootInfoFrameAllocator,
    modules::{gdt::Gdt, syscall::init_syscalls::init_syscalls},
    virt_mem_allocator::VirtMemTracker,
};

use super::handle_syscall::RustSyscallHandler;

pub const KERNEL_VIRT_MEM_START: u64 = 0xFFFF_8000_0000_0000;

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

/// # Safety
/// Literally jumps to arbitrary code. You are responsible for handling any exceptions from code / invalid code.
pub unsafe fn jmp_to_elf(
    elf_bytes: &[u8],
    mapper: Arc<spin::Mutex<OffsetPageTable<'static>>>,
    frame_allocator: Arc<spin::Mutex<BootInfoFrameAllocator>>,
    gdt: &Gdt,
    syscall_handler: RustSyscallHandler,
) -> anyhow::Result<()> {
    init_syscalls(syscall_handler);
    let elf = ElfBytes::<NativeEndian>::minimal_parse(elf_bytes)?;
    let loadable_segments = elf
        .segments()
        .ok_or(anyhow!("No segments"))?
        .into_iter()
        .filter(|segment| segment.p_type == 1)
        .collect::<Vec<_>>();

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
    // log::info!("ELF: {:#?}", loadable_segments);
    // log::info!("Symbols: {:#?}", start_symbol);

    let mut tracker = VirtMemTracker::new(VirtAddr::zero()..VirtAddr::new(KERNEL_VIRT_MEM_START));

    let mut frame_allocator = frame_allocator.lock();
    let mut mapper = mapper.lock();
    for segment in &loadable_segments {
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
        // log::info!("{:?} slice len: 0x{:x}", page_range, segment_data.len());
        tracker
            .allocate_specific_bytes_checked(
                page_range.start.start_address()..page_range.end.start_address(),
            )
            .map_err(|_| anyhow!("Failed to mark pages as used."))?;

        fn set_phys_frame(
            frame_allocator: &mut impl FrameAllocator<Size4KiB>,
            mapper: &mut impl Mapper<Size4KiB>,
            page: Page,
            f: impl FnOnce(&mut [u8]),
            final_flags: PageTableFlags,
            tracker: &mut VirtMemTracker,
        ) -> anyhow::Result<()> {
            Ok({
                let phys_frame = frame_allocator
                    .allocate_frame()
                    .ok_or(anyhow!("Failed to allocate frame"))?;
                if page.start_address().is_null() {
                    let temp_page = tracker
                        .allocate_pages::<Size4KiB>(1)
                        .ok_or(anyhow!("No page"))?;
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

                    mapper.unmap(temp_page).map_err(|_| anyhow!(""))?.1.flush();
                    tracker.deallocate_pages_unchecked(temp_page..temp_page + 1);
                    unsafe { mapper.map_to(page, phys_frame, final_flags, frame_allocator) }
                        .map_err(|_| anyhow!(""))?
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
            })
        }

        for (page_index, page) in page_range.enumerate() {
            set_phys_frame(
                frame_allocator.deref_mut(),
                mapper.deref_mut(),
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
                &mut tracker,
            )?;
        }
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

    // let start_instruction = unsafe {
    //     slice::from_raw_parts_mut::<u8>(
    //         VirtAddr::new(start_symbol.st_value).as_mut_ptr(),
    //         start_symbol.st_size as usize,
    //     )
    // };
    // You can ask GitHub Copilot to show this as assembly to verify that it's the same as what gdb shows
    // log::warn!("_start instruction: {:?}", start_instruction);

    const USER_SPACE_STACK_SIZE: usize = 0x12000;
    let page_count = (USER_SPACE_STACK_SIZE as u64).div_ceil(Size4KiB::SIZE);
    let stack_start = tracker
        .allocate_pages::<Size4KiB>(page_count)
        .ok_or(anyhow!("Failed to find pages for stack"))?;
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
                frame_allocator.deref_mut(),
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
    unsafe { enter_user_mode(gdt, start_addr, stack_end.start_address()) };

    Ok(())
}
