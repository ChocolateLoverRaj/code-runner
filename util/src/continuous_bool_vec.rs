use core::{
    cmp::Ordering,
    ops::{DerefMut, Range},
};

use crate::{insert::Insert, remove::Remove, splice::Splice};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ContinuousBoolVec<T> {
    start_value: bool,
    len_vec: T,
}

impl<T: Default + Insert<usize>> ContinuousBoolVec<T> {
    pub fn new(len: usize, start_value: bool) -> Self {
        Self {
            start_value,
            len_vec: {
                let mut len_vec = T::default();
                len_vec.insert(0, len);
                len_vec
            },
        }
    }
}

impl<T: DerefMut<Target = [usize]> + Insert<usize> + Remove<usize> + Splice<usize>>
    ContinuousBoolVec<T>
{
    pub fn set(&mut self, range: Range<usize>, value: bool) {
        let (first_segment, first_segment_position, replace_first_segment) = {
            let mut i = 0;
            let mut index = 0;
            loop {
                let end_index = index + self.len_vec[i];
                if end_index > range.start {
                    break (i, index, index == range.start);
                }
                i += 1;
                index = end_index;
            }
        };
        let (last_segment, last_segment_end_position, replace_last_segment) = {
            let mut i = first_segment;
            let mut index = 0;
            loop {
                let end_index = index + self.len_vec[i];
                if end_index >= range.end {
                    break (i, end_index, end_index == range.end);
                }
                i += 1;
                index = end_index;
            }
        };
        let (new_segment_start, new_segment_start_offset) = {
            let first_is_same = self.start_value ^ (first_segment % 2 == 1) == value;
            println!("First is same: {}", first_is_same);
            match (first_is_same, replace_first_segment) {
                (false, false) => (first_segment, range.start - first_segment_position),
                (false, true) => (first_segment.saturating_sub(1), 0),
                (true, _) => (first_segment, 0),
            }
        };
        let (new_segment_end, new_segment_end_offset) = {
            let last_is_same = self.start_value ^ (last_segment % 2 == 1) == value;
            match (last_is_same, replace_last_segment) {
                (false, false) => (last_segment, last_segment_end_position - range.end),
                (false, true) => (last_segment + 1, 0),
                (true, _) => (last_segment, 0),
            }
        };
        println!("First: {first_segment} {replace_first_segment}. Last: {last_segment} {replace_last_segment}");

        let mut new_segments = heapless::Vec::<_, 3>::new();
        if new_segment_start_offset > 0 {
            new_segments.push(new_segment_start_offset);
        }
        new_segments.push(
            (last_segment_end_position - new_segment_end_offset)
                - (first_segment_position + new_segment_start_offset),
        );
        if new_segment_end_offset > 0 {
            new_segments.push(new_segment_end_offset);
        }
        println!("{new_segment_start} {new_segment_start_offset} {new_segment_end} {new_segment_end_offset} {new_segments:?} {:?}", new_segment_start..=new_segment_end);

        self.len_vec
            .splice(new_segment_start..=new_segment_end, new_segments);
        if new_segment_start == 0 && new_segment_start_offset == 0 {
            self.start_value = value;
        }
    }
}

#[cfg(test)]
pub mod test {
    use alloc::vec::Vec;

    use super::ContinuousBoolVec;

    #[test]
    fn no_change() {
        let mut c = ContinuousBoolVec::<Vec<_>>::new(100, false);
        let old_c = c.clone();
        c.set(25..50, false);
        assert_eq!(c, old_c)
    }

    #[test]
    fn in_existing() {
        let mut c = ContinuousBoolVec::<Vec<_>>::new(100, false);
        c.set(25..50, true);
        assert_eq!(
            c,
            ContinuousBoolVec {
                start_value: false,
                len_vec: vec![25, 25, 50]
            }
        )
    }

    #[test]
    fn from_start() {
        let mut c = ContinuousBoolVec::<Vec<_>>::new(100, false);
        c.set(0..25, true);
        assert_eq!(
            c,
            ContinuousBoolVec {
                start_value: true,
                len_vec: vec![25, 75]
            }
        )
    }

    #[test]
    fn merge() {
        let mut c = ContinuousBoolVec {
            start_value: false,
            len_vec: vec![100, 100, 100],
        };
        c.set(100..200, false);
        assert_eq!(
            c,
            ContinuousBoolVec {
                start_value: false,
                len_vec: vec![300]
            }
        )
    }
}
