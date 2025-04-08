use core::ops::Range;

pub trait AlignedChunks<T> {
    fn aligned_chunks(self, alignment: T) -> impl Iterator<Item = Range<T>>;
}

impl AlignedChunks<usize> for Range<usize> {
    fn aligned_chunks(self, alignment: usize) -> impl Iterator<Item = Range<usize>> {
        RangeAlignedChunksIterator {
            alignment,
            range: self,
        }
    }
}

pub struct RangeAlignedChunksIterator {
    range: Range<usize>,
    alignment: usize,
}

impl Iterator for RangeAlignedChunksIterator {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.start == self.range.end {
            None
        } else {
            let end = (self.range.start + 1)
                .next_multiple_of(self.alignment)
                .min(self.range.end);
            if end > self.range.end {
                None
            } else {
                let range = self.range.start..end;
                self.range.start = end;
                Some(range)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty() {
        assert!((5_usize..5).aligned_chunks(10).eq([]));
    }

    #[test]
    fn partial_start_and_end() {
        assert!((2_usize..31)
            .aligned_chunks(10)
            .eq([2..10, 10..20, 20..30, 30..31]));
    }
}
