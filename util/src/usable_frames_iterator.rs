use core::ops::Range;

/// Each item is the start address of a usable frame
pub struct UsableFramesIterator<I: Iterator<Item = Range<usize>>> {
    usable_memory_regions: I,
    current_range: Option<Range<usize>>,
    index_in_frame: usize,
}

impl<I: Iterator<Item = Range<usize>>> UsableFramesIterator<I> {
    pub fn new(usable_memory_regions: I) -> Self {
        Self {
            usable_memory_regions,
            current_range: None,
            index_in_frame: 0,
        }
    }
}

impl<I: Iterator<Item = Range<usize>>> Iterator for UsableFramesIterator<I> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let region = match &self.current_range {
                Some(range) => range,
                None => self
                    .current_range
                    .insert(self.usable_memory_regions.next()?),
            };
            let frame_start = region.start.next_multiple_of(0x1000) + 0x1000 * self.index_in_frame;
            if frame_start + 0x1000 <= region.end {
                self.index_in_frame += 1;
                break Some(frame_start);
            } else {
                self.current_range = None;
                self.index_in_frame = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator() {
        let iter = UsableFramesIterator::new([0x1..0x3001, 0x5000..0x7000].into_iter());
        assert_eq!(
            iter.collect::<Vec<_>>(),
            vec![0x1000, 0x2000, 0x5000, 0x6000]
        );
    }
}
