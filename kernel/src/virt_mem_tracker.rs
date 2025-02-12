use core::{
    cmp::Ordering,
    ops::{DerefMut, Range},
};

use util::{continuous_bool_vec::ContinuousBoolVec, remove::Remove};
use x86_64::{
    structures::paging::{page::PageRange, Page, PageSize},
    VirtAddr,
};

use crate::insert::Insert;

/// All "allocation" is actually just keeping track of what's being used and not used. You have to actually do the allocating.
#[derive(Debug)]
pub struct VirtMemTracker {
    starting_addr: VirtAddr,
    used_addresses: ContinuousBoolVec<heapless::Vec<usize, 50>>,
}

impl VirtMemTracker {
    pub fn new(addr_range: Range<VirtAddr>) -> Self {
        Self {
            starting_addr: addr_range.start,
            used_addresses: ContinuousBoolVec::new(
                (addr_range.end - addr_range.start) as usize,
                false,
            ),
        }
    }
    /// Finds available bytes and allocates them
    pub fn allocate_bytes(&mut self, len: u64) -> Option<VirtAddr> {
        Some({
            let range_start = self
                .used_addresses
                .get_continuous_range(false, len as usize)?;
            self.used_addresses
                .set(range_start..range_start + len as usize, true);
            self.starting_addr + range_start as u64
        })
    }

    pub fn allocate_pages<S: PageSize>(&mut self, page_count: u64) -> Option<Page<S>> {
        Some({
            let bytes_len = (S::SIZE * page_count) as usize;
            let range_start = self.used_addresses.get_continuous_range_with_alignment(
                false,
                bytes_len,
                S::SIZE as usize,
            )?;
            // log::info!("Before set: {:#?}", self.used_addresses);
            self.used_addresses
                .set(range_start..range_start + bytes_len, true);
            // log::info!("After set: {:#?}", self.used_addresses);
            Page::from_start_address(self.starting_addr + range_start as u64).unwrap()
        })
    }

    /// This does not check if the bytes are already allocated
    pub fn allocate_specific_bytes_unchecked(&mut self, range: Range<VirtAddr>) {
        let range_to_set =
            (range.start - self.starting_addr) as usize..(range.end - self.starting_addr) as usize;
        self.used_addresses.set(range_to_set, true);
    }

    /// This makes sure that the specific bytes are not in use before allocating
    #[allow(clippy::result_unit_err)]
    pub fn allocate_specific_bytes_checked(&mut self, range: Range<VirtAddr>) -> Result<(), ()> {
        let range_to_set =
            (range.start - self.starting_addr) as usize..(range.end - self.starting_addr) as usize;
        if self
            .used_addresses
            .is_range_available(false, range_to_set.clone())
        {
            self.allocate_specific_bytes_unchecked(range);
            Ok(())
        } else {
            Err(())
        }
    }

    /// This does not check if you are accidentally deallocating bytes that you didn't allocate in the first place
    pub fn deallocate_bytes_unchecked(&mut self, range: Range<VirtAddr>) {
        let range_to_set =
            (range.start - self.starting_addr) as usize..(range.end - self.starting_addr) as usize;
        // log::info!(
        //     "[deallocate] deallocating: {:?} before set: {:#?}",
        //     range_to_set,
        //     self.used_addresses
        // );
        self.used_addresses.set(range_to_set, false);
        // log::info!("[deallocate] after set: {:#?}", self.used_addresses);
    }

    pub fn deallocate_pages_unchecked<S: PageSize>(&mut self, pages: Range<Page<S>>) {
        self.deallocate_bytes_unchecked(pages.start.start_address()..pages.end.start_address());
    }
}

// TODO: Tests (very ez and very important for this)
pub trait VirtMemAllocator {
    fn allocate_pages<S: PageSize>(
        &mut self,
        page_count: u64,
        start_at: VirtAddr,
    ) -> Option<VirtAddr>;
    /// Deallocates pages, if they are allocated. It doesn't check if ur trying to deallocate unallocated pages but if you try doing that there is probably a bug in your code.
    fn deallocate_pages<S: PageSize>(&mut self, page_range: PageRange<S>);
    fn allocate_specific(&mut self, page_range: Range<VirtAddr>) -> Option<()>;
}

impl<
        T: DerefMut<Target = [Range<VirtAddr>]> + Insert<Range<VirtAddr>> + Remove<Range<VirtAddr>>,
    > VirtMemAllocator for T
{
    fn allocate_pages<S: PageSize>(
        &mut self,
        page_count: u64,
        start_at: VirtAddr,
    ) -> Option<VirtAddr> {
        let (index, start) = {
            // 0 cannot be used since that's reserved for a null pointer
            let mut start = start_at;
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

    fn allocate_specific(&mut self, range: Range<VirtAddr>) -> Option<()> {
        let mut i = 1;
        loop {
            match self.get(i) {
                Some(existing_range) => match existing_range.start.cmp(&range.start) {
                    Ordering::Less => match existing_range.end.cmp(&range.start) {
                        Ordering::Less => {
                            i += 1;
                        }
                        Ordering::Equal => match self.get(i + 1) {
                            Some(next_range) => match next_range.start.cmp(&range.end) {
                                Ordering::Less => break None,
                                Ordering::Equal => {
                                    self[i + 1].start = range.start;
                                    break Some(());
                                }
                                Ordering::Greater => {
                                    self.insert(i + 1, range);
                                    break Some(());
                                }
                            },
                            None => {
                                self.insert(i + 1, range);
                                break Some(());
                            }
                        },
                        Ordering::Greater => break None,
                    },
                    Ordering::Equal => break None,
                    Ordering::Greater => match existing_range.start.cmp(&range.end) {
                        Ordering::Less => break None,
                        Ordering::Equal => {
                            self[i].start = range.start;
                        }
                        Ordering::Greater => {
                            self.insert(i, range);
                            break Some(());
                        }
                    },
                },
                None => {
                    self.insert(i, range);
                    break Some(());
                }
            }
        }
    }
}
