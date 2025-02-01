use core::{
    cmp::Ordering,
    ops::{Deref, DerefMut, Range},
};

use super::ContinuousBoolVec;

impl<T: Deref<Target = [usize]>> ContinuousBoolVec<T> {
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
                        break Some(
                            current_segment_start_pos..current_segment_start_pos + requested_len,
                        );
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
        assert_eq!(r, Some(0..50))
    }

    #[test]
    fn next() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![50, 50],
        };
        let r = c.get_continuous_range(false, 50);
        assert_eq!(r, Some(50..100))
    }

    #[test]
    fn skip() {
        let c = ContinuousBoolVec {
            start_value: true,
            len_vec: vec![25, 25, 50, 50],
        };
        let r = c.get_continuous_range(false, 50);
        assert_eq!(r, Some(100..150))
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
}
