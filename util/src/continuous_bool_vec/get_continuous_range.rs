use core::{
    cmp::Ordering,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
};

use super::ContinuousBoolVec;

impl<T: Deref<Target = [usize]>> ContinuousBoolVec<T> {
    pub fn get_continuous_range(&self, value: bool, requested_len: usize) -> Option<usize> {
        let mut current_segment_value = self.start_value;
        let mut i = 0;
        let mut current_segment_start_pos = 0;
        loop {
            match self.len_vec.get(i) {
                Some(len) => {
                    let len = *len;
                    let current_segment_end_pos = current_segment_start_pos + len;
                    if current_segment_value == value && len >= requested_len {
                        break Some(current_segment_start_pos);
                    }
                    i += 1;
                    current_segment_value = !current_segment_value;
                    current_segment_start_pos = current_segment_end_pos;
                }
                None => break None,
            }
        }
    }

    pub fn get_continuous_range_with_alignment(
        &self,
        value: bool,
        requested_len: usize,
        alignment: usize,
    ) -> Option<usize> {
        let mut current_segment_value = self.start_value;
        let mut i = 0;
        let mut current_segment_start_pos = 0;
        loop {
            match self.len_vec.get(i) {
                Some(len) => {
                    let len = *len;
                    let current_segment_end_pos = current_segment_start_pos + len;
                    let aligned_start = round_mult::up(
                        current_segment_start_pos,
                        NonZeroUsize::new(alignment).unwrap(),
                    )
                    .unwrap();
                    if current_segment_value == value
                        && current_segment_end_pos - aligned_start > requested_len
                    {
                        break Some(aligned_start);
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
    use super::ContinuousBoolVec;

    #[test]
    fn blank() {
        let c = ContinuousBoolVec {
            start_value: false,
            len_vec: vec![100],
        };
        let r = c.get_continuous_range(false, 50);
        assert_eq!(r, Some(0))
    }

    #[test]
    fn next() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![50, 50],
        };
        let r = c.get_continuous_range(false, 50);
        assert_eq!(r, Some(50))
    }

    #[test]
    fn skip() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![25, 25, 50, 50],
        };
        let r = c.get_continuous_range(false, 50);
        assert_eq!(r, Some(100))
    }

    #[test]
    fn no_space() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![25, 25, 25, 25],
        };
        let r = c.get_continuous_range(false, 50);
        assert_eq!(r, None)
    }

    #[test]
    fn no_space_cuz_of_alignment() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![1, 4],
        };
        let r = c.get_continuous_range_with_alignment(false, 4, 2);
        assert_eq!(r, None)
    }

    #[test]
    fn alignment() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![1, 4],
        };
        let r = c.get_continuous_range_with_alignment(false, 2, 2);
        assert_eq!(r, Some(2))
    }
}
