use core::{
    cmp::Ordering,
    ops::{DerefMut, Range},
};

use x86_64::{
    structures::paging::{page::PageRange, PageSize},
    VirtAddr,
};

use crate::{insert::Insert, remove::Remove};

// TODO: Tests (very ez and very important for this)
pub trait VirtMemAllocator {
    fn allocate_pages<S: PageSize>(&mut self, page_count: u64) -> Option<VirtAddr>;
    /// Deallocates pages, if they are allocated. It doesn't check if ur trying to deallocate unallocated pages but if you try doing that there is probably a bug in your code.
    fn deallocate_pages<S: PageSize>(&mut self, page_range: PageRange<S>);
}

impl<
        T: DerefMut<Target = [Range<VirtAddr>]> + Insert<Range<VirtAddr>> + Remove<Range<VirtAddr>>,
    > VirtMemAllocator for T
{
    fn allocate_pages<S: PageSize>(&mut self, page_count: u64) -> Option<VirtAddr> {
        let (index, start) = {
            // 0 cannot be used since that's reserved for a null pointer
            let mut start = VirtAddr::new((0 + 1) * S::SIZE);
            let mut iter = self.iter().enumerate();
            loop {
                match iter.next() {
                    Some((index, range)) => {
                        if start + S::SIZE * page_count <= range.start {
                            break Some((index, start));
                        }
                        start = range.end.align_up(S::SIZE);
                    }
                    None => {
                        let max_virt_address_excluding = 1 << 48;
                        if start.as_u64() + S::SIZE * page_count <= max_virt_address_excluding {
                            break Some((self.len(), start));
                        } else {
                            break None;
                        }
                    }
                }
            }
        }?;
        // TODO: Optimize
        // Extend an existing range if possible, otherwise insert a new range while keeping the list sorted
        let end = start + S::SIZE * page_count;
        if let Some(prev_range) = index.checked_sub(1).and_then(|prev_index| {
            self.get_mut(prev_index)
                .filter(|prev_range| prev_range.end == start)
        }) {
            prev_range.end = end;
        } else if let Some(next_range) = self
            .get_mut(index + 1)
            .filter(|next_range| next_range.start == end)
        {
            next_range.start = start;
        } else {
            self.insert(index, Range { start, end });
        }
        Some(start)
    }

    fn deallocate_pages<S: PageSize>(&mut self, page_range: PageRange<S>) {
        let range_to_deallocate = Range {
            start: page_range.start.start_address(),
            end: page_range.end.start_address(),
        };
        let mut i = 0;
        loop {
            let range = &mut self[i];
            if range.start >= range_to_deallocate.end {
                break;
            }
            match range.start.cmp(&range_to_deallocate.start) {
                Ordering::Less => {
                    match range.end.cmp(&range_to_deallocate.start) {
                        Ordering::Less | Ordering::Equal => {
                            // No overlap, do nothing
                            i += 1;
                        }
                        Ordering::Greater => {
                            if range.end > range_to_deallocate.end {
                                // This chunk will be split into two chunks with a gap in the middle
                                let new_range = Range {
                                    start: range_to_deallocate.end,
                                    end: range.end,
                                };
                                self.insert(i + 1, new_range);
                            }
                            // Slice right end
                            let range = &mut self[i];
                            range.end = range_to_deallocate.start;
                            i += 1;
                        }
                    }
                }
                Ordering::Equal | Ordering::Greater => {
                    match range.end.cmp(&range_to_deallocate.end) {
                        Ordering::Less | Ordering::Equal => {
                            // Remove the whole chunk
                            self.remove(i);
                        }
                        Ordering::Greater => {
                            // Slice left end
                            range.start = range_to_deallocate.end;
                            i += 1;
                        }
                    }
                }
            }
            if i >= self.len() {
                break;
            }
        }
    }
}
