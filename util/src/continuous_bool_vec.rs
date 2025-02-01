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
    pub fn set(&mut self, mut range: Range<usize>, value: bool) {
        let mut i = 0;
        let mut current_segment_start_pos = 0;
        let mut current_segment_value = self.start_value;

        if range.start == 0 {
            self.start_value = value;
        }

        loop {
            let current_segment_len = self.len_vec[i];
            let current_segment_end_pos = current_segment_start_pos + current_segment_len;
            if current_segment_end_pos >= range.start {
                // println!("Here. i: {i}. current segment value: {current_segment_value}.");
                let mut increased_by = if current_segment_value == value {
                    // println!("here 2");
                    // Merge with previous
                    range.start = current_segment_start_pos;
                    let extend_by = range.end as isize - current_segment_end_pos as isize;
                    if extend_by <= 0 {
                        // No change
                        break;
                    } else {
                        let extend_by = extend_by as usize;
                        self.len_vec[i] += extend_by;
                        i += 1;
                        extend_by
                    }
                } else {
                    // Cut off the right part of the current segment
                    self.len_vec[i] = range.start - current_segment_start_pos;
                    if self.len_vec[i] == 0 {
                        self.len_vec.remove(i);
                    } else {
                        i += 1;
                    }
                    let right_part_of_cut_current_segment = current_segment_end_pos
                        .checked_sub(range.end)
                        .and_then(|size| match size {
                            0 => None,
                            size => Some(size),
                        });
                    self.len_vec.insert(i, range.len());
                    i += 1;
                    current_segment_value = !current_segment_value;
                    if let Some(right_part_of_cut_current_segment) =
                        right_part_of_cut_current_segment
                    {
                        self.len_vec.insert(i, right_part_of_cut_current_segment);
                        i += 1;
                        current_segment_value = !current_segment_value;
                        break;
                    } else {
                        let increased_by = range.end - current_segment_end_pos;
                        increased_by
                    }
                };
                // println!("Range: {range:?}. Increased by: {increased_by}. I: {i}");
                loop {
                    let current_segment_len = self.len_vec[i];
                    let decrease_by = current_segment_len.min(increased_by);
                    self.len_vec[i] -= decrease_by;
                    if self.len_vec[i] == 0 {
                        self.len_vec.remove(i);
                    } else {
                        i += 1;
                    }
                    increased_by -= decrease_by;
                    if increased_by == 0 {
                        break;
                    }
                }
                if let Some(current_segment_len) = self.len_vec.get(i) {
                    let current_segment_len = *current_segment_len;
                    if current_segment_value == value {
                        self.len_vec.remove(i);
                        self.len_vec[i - 1] += current_segment_len;
                    }
                }
                break;
            } else {
                i += 1;
                current_segment_start_pos = current_segment_end_pos;
                current_segment_value = !current_segment_value;
            }
        }
    }

    pub fn get_continuous_range(&self, value: bool, requested_len: usize) -> Option<Range<usize>> {
        let mut current_segment_value = self.start_value;
        let mut i = 0;
        let mut current_segment_start_pos = 0;
        loop {
            match self.len_vec.get(i) {
                Some(len) => {
                    let len = *len;
                    let current_segment_end_pos = current_segment_start_pos + len;
                    if current_segment_value == value && len >= requested_len {
                        break Some(current_segment_start_pos..current_segment_end_pos);
                    }
                    i += 1;
                    current_segment_value = !current_segment_value;
                    current_segment_start_pos = current_segment_end_pos;
                }
                None => break None,
            }
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

    #[test]
    fn overwrite() {
        let mut c = ContinuousBoolVec {
            start_value: false,
            len_vec: vec![100, 100],
        };
        c.set(0..200, true);
        assert_eq!(
            c,
            ContinuousBoolVec {
                start_value: true,
                len_vec: vec![200]
            }
        )
    }

    #[test]
    fn complex() {
        let mut c = ContinuousBoolVec {
            start_value: false,
            len_vec: vec![100, 100, 100, 100],
        };
        c.set(50..350, true);
        assert_eq!(
            c,
            ContinuousBoolVec {
                start_value: false,
                len_vec: vec![50, 300, 50]
            }
        )
    }

    #[test]
    fn complex2() {
        let mut c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![100, 100, 100, 100],
        };
        c.set(100..300, false);
        assert_eq!(
            c,
            ContinuousBoolVec {
                start_value: true,
                len_vec: vec![100, 300]
            }
        )
    }
}
