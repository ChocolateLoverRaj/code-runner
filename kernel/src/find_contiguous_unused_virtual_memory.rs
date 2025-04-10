use core::ops::Range;

use x86_64::structures::paging::{Page, Size4KiB};

use crate::page_tables_recursive_iterator::PageTablesRecursiveIterator;

/// Only finds contiguous memory in the higher half of the virtual address space.
pub fn find_contiguous_unused_virtual_memory(
    mut iterator: PageTablesRecursiveIterator,
    n_4kib_pages: u64,
) -> Option<Range<Page<Size4KiB>>> {
    let mut contiguous_unused = None;
    loop {
        let entry = iterator.next()?;
        if entry.present {
            contiguous_unused = None;
        } else {
            match &mut contiguous_unused {
                Some((start, contiguous_n_4kib_pages)) => {
                    *contiguous_n_4kib_pages += entry.page_table_index_stack.n_4kib_pages();
                    if *contiguous_n_4kib_pages >= n_4kib_pages {
                        return Some(*start..*start + n_4kib_pages);
                    }
                }
                None => {
                    contiguous_unused = Some((
                        Page::<Size4KiB>::from_start_address(
                            entry.page_table_index_stack.start_address(),
                        )
                        .unwrap(),
                        entry.page_table_index_stack.n_4kib_pages(),
                    ));
                }
            }
        }
    }
}
