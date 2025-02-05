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
        // log::info!("{:?} slice len: 0x{:x}", page_range, segment_data.len());
        tracker
            .allocate_specific_bytes_checked(
                page_range.start.start_address()..page_range.end.start_address(),
            )
            .map_err(|_| anyhow!("Failed to mark pages as used."))?;
        for (page_index, page) in page_range.enumerate() {
            let phys_frame = frame_allocator
                .allocate_frame()
                .ok_or(anyhow!("Failed to allocate frame"))?;

            unsafe {
                mapper.map_to(
                    page,
                    phys_frame,
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        // FIXME: Remove user accessible. But for now, we keep it cuz of [a bug in `update_flags`](https://github.com/rust-osdev/x86_64/issues/534)
                        | PageTableFlags::USER_ACCESSIBLE,
                    frame_allocator.deref_mut(),
                )
            }
            .map_err(|_| anyhow!("Failed to map page"))?
            .flush();

            // FIXME: Rust cannot work with the page at 0x0 cuz Rust has errors on "null pointers"
            if page.start_address() == VirtAddr::zero() {
                continue;
            }
            let slice = unsafe {
                slice::from_raw_parts_mut::<u8>(
                    page.start_address().as_mut_ptr(),
                    page.size() as usize,
                )
            };
            // log::info!("Slice: {:?}. Len should be: {}", slice, page.size());
            // Zero the phys frame to be secure
            slice.fill(Default::default());
            // Copy the data
            let start = segment.p_vaddr % segment.p_align;
            let end = (start + (segment.p_filesz - Size4KiB::SIZE * page_index as u64))
                .min(slice.len() as u64);

            let src_start = Size4KiB::SIZE * page_index as u64;
            let src_end = src_start
                + (segment.p_filesz - Size4KiB::SIZE * page_index as u64).min(Size4KiB::SIZE);
            // log::warn!(
            //     "Copying to frame: {:?} from segment data: {:?}",
            //     start..end,
            //     src_start..src_end,
            // );
            slice[start as usize..end as usize]
                .copy_from_slice(&segment_data[src_start as usize..src_end as usize]);
            // Set flags for user space
            let flags = PageTableFlags::PRESENT
                | PageTableFlags::USER_ACCESSIBLE
                | elf_flags_to_page_table_flags(segment.p_flags);
            // log::info!("setting flags for {page:?}: {flags:?}");
            unsafe { mapper.update_flags(page, flags) }
                .map_err(|_| anyhow!("Failed to update flags"))?
                .flush();
        }
    }
    // let start_instruction = slice::from_raw_parts_mut::<u8>(
    //     VirtAddr::new(start_symbol.st_value).as_mut_ptr(),
    //     start_symbol.st_size as usize,
    // );
    // You can ask GitHub Copilot to show this as assembly to verify that it's the same as what gdb shows
    // log::warn!("_start instruction: {:?}", start_instruction);

    const USER_SPACE_STACK_SIZE: usize = 0x1000;
    let page_count = (USER_SPACE_STACK_SIZE as u64).div_ceil(Size4KiB::SIZE);
    let stack_start = tracker
        .allocate_pages::<Size4KiB>(page_count)
        .ok_or(anyhow!("Failed to find pages for stack"))?;
    let stack_end = stack_start + page_count;
    let stack_pages = stack_start..stack_end;
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
